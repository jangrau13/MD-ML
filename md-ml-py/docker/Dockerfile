FROM python:3.12-slim

RUN pip install --no-cache-dir numpy>=1.24

WORKDIR /app
COPY md_ml/ /app/md_ml/
COPY docker/ /app/docker/

EXPOSE 8080
