# Tsurust Deployment Guide

This guide covers deploying Tsurust (server + WASM client) to a VPC or cloud environment using Docker containers.

## Architecture Overview

- **Server**: Rust WebSocket server (port 8080)
- **Client**: WASM application served by nginx (port 80)
- **Communication**: WebSocket protocol between client and server

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+ (for local testing)
- For VPC deployment: Container orchestration platform (ECS, Kubernetes, etc.)

## Local Development with Docker

### Build and Run with Docker Compose

```bash
# Build and start both services
docker-compose up --build

# Access the application
# Client: http://localhost
# Server WebSocket: ws://localhost:8080
```

### Environment Variables

#### Server (`tsurust-server`)
- `HOST`: Bind address (default: `0.0.0.0` in containers, `127.0.0.1` native)
- `PORT`: WebSocket server port (default: `8080`)

#### Client (`tsurust-client`)
- `WS_SERVER_URL`: WebSocket server URL (default: `ws://localhost:8080`)
  - Local: `ws://localhost:8080`
  - Production: `wss://your-domain.com` (requires SSL/TLS)

### Build Individual Images

```bash
# Build server image
docker build -f server/Dockerfile -t tsurust-server:latest .

# Build client image
docker build -f client-egui/Dockerfile -t tsurust-client:latest .
```

### Run Individual Containers

```bash
# Run server
docker run -d \
  --name tsurust-server \
  -p 8080:8080 \
  -e HOST=0.0.0.0 \
  -e PORT=8080 \
  tsurust-server:latest

# Run client
docker run -d \
  --name tsurust-client \
  -p 80:80 \
  -e WS_SERVER_URL=ws://localhost:8080 \
  tsurust-client:latest
```

## VPC Deployment

### AWS ECS Deployment

#### 1. Push Images to ECR

```bash
# Authenticate with ECR
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin <account-id>.dkr.ecr.us-east-1.amazonaws.com

# Tag images
docker tag tsurust-server:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/tsurust-server:latest
docker tag tsurust-client:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/tsurust-client:latest

# Push images
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/tsurust-server:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/tsurust-client:latest
```

#### 2. Create ECS Task Definitions

**Server Task Definition** (`ecs-server-task.json`):
```json
{
  "family": "tsurust-server",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "256",
  "memory": "512",
  "containerDefinitions": [
    {
      "name": "tsurust-server",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/tsurust-server:latest",
      "essential": true,
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "HOST", "value": "0.0.0.0"},
        {"name": "PORT", "value": "8080"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/tsurust-server",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "timeout 2 bash -c '</dev/tcp/localhost/8080' || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 10
      }
    }
  ]
}
```

**Client Task Definition** (`ecs-client-task.json`):
```json
{
  "family": "tsurust-client",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "256",
  "memory": "512",
  "containerDefinitions": [
    {
      "name": "tsurust-client",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/tsurust-client:latest",
      "essential": true,
      "portMappings": [
        {
          "containerPort": 80,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "WS_SERVER_URL", "value": "wss://ws.yourdomain.com"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/tsurust-client",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

#### 3. Create ECS Services

```bash
# Create server service
aws ecs create-service \
  --cluster tsurust-cluster \
  --service-name tsurust-server \
  --task-definition tsurust-server \
  --desired-count 1 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-xxx],securityGroups=[sg-xxx],assignPublicIp=ENABLED}"

# Create client service
aws ecs create-service \
  --cluster tsurust-client \
  --service-name tsurust-client \
  --task-definition tsurust-client \
  --desired-count 1 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-xxx],securityGroups=[sg-xxx],assignPublicIp=ENABLED}"
```

#### 4. Configure Load Balancer

- **Application Load Balancer (ALB)** for HTTP/HTTPS traffic to client
- **Network Load Balancer (NLB)** for WebSocket traffic to server
- Configure SSL/TLS certificates via ACM for production

### Kubernetes Deployment

#### Server Deployment (`k8s/server-deployment.yaml`):
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tsurust-server
spec:
  replicas: 2
  selector:
    matchLabels:
      app: tsurust-server
  template:
    metadata:
      labels:
        app: tsurust-server
    spec:
      containers:
      - name: server
        image: tsurust-server:latest
        ports:
        - containerPort: 8080
        env:
        - name: HOST
          value: "0.0.0.0"
        - name: PORT
          value: "8080"
        livenessProbe:
          tcpSocket:
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          tcpSocket:
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: tsurust-server
spec:
  selector:
    app: tsurust-server
  ports:
  - protocol: TCP
    port: 8080
    targetPort: 8080
  type: LoadBalancer
```

#### Client Deployment (`k8s/client-deployment.yaml`):
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tsurust-client
spec:
  replicas: 2
  selector:
    matchLabels:
      app: tsurust-client
  template:
    metadata:
      labels:
        app: tsurust-client
    spec:
      containers:
      - name: client
        image: tsurust-client:latest
        ports:
        - containerPort: 80
        env:
        - name: WS_SERVER_URL
          value: "wss://ws.yourdomain.com"
        livenessProbe:
          httpGet:
            path: /health
            port: 80
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 80
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: tsurust-client
spec:
  selector:
    app: tsurust-client
  ports:
  - protocol: TCP
    port: 80
    targetPort: 80
  type: LoadBalancer
```

## SSL/TLS Configuration

For production deployments, use HTTPS/WSS:

1. **Obtain SSL certificates** (Let's Encrypt, ACM, etc.)
2. **Configure reverse proxy** (nginx, traefik, ALB):
   - Client: HTTPS (port 443) → Container (port 80)
   - Server: WSS (port 443) → Container (port 8080)
3. **Update client environment**:
   ```bash
   WS_SERVER_URL=wss://ws.yourdomain.com
   ```

### Example nginx SSL Configuration

```nginx
# /etc/nginx/sites-available/tsurust
server {
    listen 443 ssl http2;
    server_name app.yourdomain.com;

    ssl_certificate /etc/ssl/certs/yourdomain.crt;
    ssl_certificate_key /etc/ssl/private/yourdomain.key;

    location / {
        proxy_pass http://tsurust-client:80;
    }
}

server {
    listen 443 ssl http2;
    server_name ws.yourdomain.com;

    ssl_certificate /etc/ssl/certs/yourdomain.crt;
    ssl_certificate_key /etc/ssl/private/yourdomain.key;

    location / {
        proxy_pass http://tsurust-server:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

## Monitoring and Logging

### Health Checks
- **Server**: TCP connection to port 8080
- **Client**: HTTP GET `/health` endpoint

### Logs
- Server logs go to stdout/stderr (captured by Docker/K8s)
- Client nginx access/error logs to stdout/stderr
- Use CloudWatch Logs (AWS), Google Cloud Logging, or ELK stack

### Metrics
Consider adding:
- Prometheus metrics export
- CloudWatch metrics
- Application performance monitoring (APM)

## Security Considerations

1. **Run as non-root user** (already configured in Dockerfiles)
2. **Use secrets management** for sensitive configuration
3. **Enable CORS** properly (update nginx.conf for production domains)
4. **Rate limiting** on WebSocket connections
5. **Network policies** to restrict container communication
6. **Regular security updates** of base images

## Troubleshooting

### Client can't connect to server
- Check `WS_SERVER_URL` environment variable
- Verify network policies allow WebSocket connections
- Check firewall rules for port 8080
- Ensure SSL/TLS is properly configured for WSS

### Container won't start
- Check logs: `docker logs <container-name>`
- Verify health checks: `docker inspect <container-name>`
- Ensure ports aren't already in use

### Build failures
- Clear Docker cache: `docker system prune -a`
- Check Rust version compatibility
- Verify all source files are included (check .dockerignore)

## Scaling

- **Server**: Can scale horizontally, but requires session affinity or state synchronization
- **Client**: Fully stateless, can scale horizontally without limits
- Consider adding Redis for distributed session management

## Cost Optimization

- Use multi-stage builds (already configured) to minimize image size
- Server image: ~50-100 MB
- Client image: ~20-30 MB (nginx + WASM assets)
- Consider spot instances for dev/staging environments

## Next Steps

- [ ] Set up CI/CD pipeline (GitHub Actions, GitLab CI, etc.)
- [ ] Configure auto-scaling based on metrics
- [ ] Add distributed session storage (Redis)
- [ ] Implement monitoring and alerting
- [ ] Set up backup and disaster recovery
