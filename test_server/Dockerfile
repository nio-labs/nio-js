FROM ubuntu:24.04
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://nio.dev/install.sh | bash
COPY . /app
WORKDIR /app
CMD ["nio-js", "task", "start"]
