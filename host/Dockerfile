# Stage 1: Build Frontend
FROM node:20-slim as frontend-builder
WORKDIR /app
# Copy dependency definitions
COPY package.json package-lock.json ./
RUN npm ci

# Copy source code
COPY . .
# Build using the host config to enable Web Mode
RUN npx vite build --config host/vite.config.ts

# Stage 2: Runtime
FROM python:3.11-slim
WORKDIR /app

# Install system dependencies for audio analysis and processing
RUN apt-get update && apt-get install -y \
    ffmpeg \
    libsndfile1 \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY host/backend/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Attempt to install Essentia (TensorFlow version usually has wheels)
# If it fails (e.g. on ARM/Apple Silicon), we continue as we have Librosa fallback
RUN pip install --no-cache-dir essentia-tensorflow || pip install --no-cache-dir essentia || echo "Essentia installation failed. Will use Librosa fallback."

# Copy Backend Code
COPY host/backend /app/backend

# Copy Analysis Script (Reusable Logic)
# We place it in backend/services so it can be imported or run
COPY src-tauri/sidecar/analyze_track.py /app/backend/services/analyze_track.py

# Copy Built Frontend from Stage 1
COPY --from=frontend-builder /app/host/dist /app/dist

# Environment variables
ENV PYTHONUNBUFFERED=1
ENV DIST_DIR=/app/dist

# Expose port
EXPOSE 8000

# Run the application
CMD ["uvicorn", "backend.main:app", "--host", "0.0.0.0", "--port", "8000"]
