import { motion } from 'motion/react';
import React, { useMemo } from 'react';

interface CurvedListProps<T> {
    items: T[];
    renderItem: (item: T, index: number, isActive: boolean) => React.ReactNode;
    activeIndex: number;
    side: 'left' | 'right';
    radius?: number;
    itemHeight?: number; // Approximate height for spacing (degrees)
    visibleRange?: number; // How many items to show above/below
    className?: string;
}

const CurvedListInner = <T,>({
    items,
    renderItem,
    activeIndex,
    side,
    radius = 400,
    itemHeight = 40,
    visibleRange = 5,
    className = ""
}: CurvedListProps<T>) => {

    // Calculate the angle step based on item height and radius
    // angle (degrees) ~ (arcLength / radius) * (180/PI)
    // Reduce the step slightly to make items tighter
    const angleStep = (itemHeight / radius) * (180 / Math.PI) * 1.0;

    const visibleItems = useMemo(() => {
        const start = Math.max(0, activeIndex - visibleRange);
        const end = Math.min(items.length - 1, activeIndex + visibleRange);
        const result = [];
        for (let i = start; i <= end; i++) {
            result.push({ item: items[i], originalIndex: i });
        }
        return result;
    }, [items, activeIndex, visibleRange]);

    return (
        <div className={`relative h-full w-full flex items-center justify-center pointer-events-none ${className}`}>
            {visibleItems.map(({ item, originalIndex }) => {
                const offsetIndex = originalIndex - activeIndex;

                const baseAngle = side === 'left' ? 180 : 0;
                const angle = baseAngle + (offsetIndex * angleStep);

                const isActive = originalIndex === activeIndex;

                return (
                    // Pivot Layer: Rotates to the correct angle on the circle
                    <motion.div
                        key={originalIndex}
                        className="absolute flex items-center justify-center"
                        style={{
                            left: '50%',
                            top: '50%',
                            width: 0,
                            height: 0
                        }}
                        initial={false}
                        animate={{ rotate: angle, zIndex: isActive ? 50 : 10 - Math.abs(offsetIndex) }}
                        transition={{ type: "spring", stiffness: 300, damping: 30 }}
                    >
                        {/* 
                            Radius Layer: Pushes Item out to the radius.
                        */}
                        <motion.div
                            style={{ x: radius }}
                        >
                            {/* 
                                Counter-Rotate Layer: Keeps text horizontal.
                            */}
                            <motion.div
                                animate={{ rotate: -angle }}
                                transition={{ type: "spring", stiffness: 300, damping: 30 }}
                                className={`flex items-center pointer-events-auto origin-center`}
                                style={{
                                    width: '500px', // Wide container to catch long titles
                                    height: '80px',
                                    // Align text based on side
                                    // Side Left: Text should End at the radius pivot. -> justify-end.
                                    // Side Right: Text should Start at the radius pivot. -> justify-start.
                                    justifyContent: side === 'left' ? 'flex-end' : 'flex-start',

                                    // Adjustment: The pivot is the Center of this div.
                                    // If we want the text edge to be at radius, we need to shift the div.
                                    // Since width=500, center is 250.
                                    // Left side: We want Right Edge (500) at 0 relative to pivot. So translateX(-250).
                                    // Right side: We want Left Edge (0) at 0 relative to pivot. So translateX(250).

                                    transform: `translateX(${side === 'left' ? '-50%' : '50%'})`
                                }}
                            >
                                <motion.div
                                    className="w-full"
                                    animate={{
                                        opacity: 1 - (Math.abs(offsetIndex) / (visibleRange + 1)),
                                        scale: isActive ? 1.05 : 0.95,
                                    }}
                                >
                                    {renderItem(item, originalIndex, isActive)}
                                </motion.div>
                            </motion.div>
                        </motion.div>
                    </motion.div>
                );
            })}
        </div>
    );
};

export const CurvedList = React.memo(CurvedListInner) as typeof CurvedListInner;
