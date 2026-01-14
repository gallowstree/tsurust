# Tsurust Production Deployment Proposal

**Date:** 2026-01-14
**Version:** 1.0
**Status:** Draft for Review

---

## Executive Summary

This proposal outlines the requirements and steps to prepare the Tsurust multiplayer board game for production deployment. The application consists of three components: a Rust WebSocket server, a WASM browser client, and native desktop clients. This document provides a roadmap to achieve production-readiness with emphasis on reliability, security, and user experience.

---

## Current State Assessment

### ✅ What's Working Well

**Core Functionality:**
- ✅ Fully playable game with complete rule implementation
- ✅ Local multiplayer (hot-seat mode)
- ✅ Online multiplayer via WebSocket
- ✅ Lobby system with room codes
- ✅ Player statistics and game export/import
- ✅ Replay system with playback controls
- ✅ Native (desktop) and WASM (browser) clients
- ✅ 74 passing tests across all packages

**Recent Improvements:**
- ✅ Fixed macOS crash (upgraded egui 0.26 → 0.29)
- ✅ Fixed player color persistence bug
- ✅ Docker support for both client and server
- ✅ Animated UI effects (gradient border glow)

### 🔴 Critical Gaps for Production

1. **Testing Coverage**
   - No integration tests for online multiplayer
   - No serialization tests for protocol messages
   - No load testing or stress testing
   - No automated end-to-end tests

2. **Server Reliability**
   - No disconnect/reconnect handling
   - No room cleanup for abandoned games
   - No heartbeat/health checks
   - No rate limiting or DDoS protection
   - No logging/monitoring infrastructure

3. **Security**
   - No authentication or authorization
   - No input validation/sanitization
   - No rate limiting
   - No HTTPS/WSS (secure WebSocket)
   - No CORS configuration for production

4. **Deployment Infrastructure**
   - No CI/CD pipeline
   - No production environment configuration
   - No database for persistent state
   - No load balancing or horizontal scaling
   - No backup/disaster recovery plan

5. **User Experience**
   - No error messages for network failures
   - No loading states or progress indicators
   - No connection status indicators
   - No user onboarding or tutorial
   - No mobile responsiveness

---

## Production Readiness Roadmap

### Phase 1: Critical Stability & Testing (2-3 weeks)

**Priority: CRITICAL** - Cannot deploy without these

#### 1.1 Testing Infrastructure
**Estimated Time:** 1 week

- [ ] **Integration Tests for Multiplayer**
  - Test tile placement synchronization
  - Test lobby flow (create, join, start)
  - Test game state consistency across clients
  - Test player movement and elimination
  - Test game completion and statistics

- [ ] **Protocol Serialization Tests**
  - Serialize/deserialize all ClientMessage variants
  - Serialize/deserialize all ServerMessage variants
  - Test error handling for malformed messages
  - Prevent regression of JSON key errors

- [ ] **Load Testing**
  - Test server with 10, 50, 100 concurrent games
  - Measure memory usage and CPU under load
  - Identify bottlenecks and performance issues
  - Document resource requirements

**Acceptance Criteria:**
- 90%+ code coverage for core game logic
- All protocol messages tested for round-trip serialization
- Server handles 50+ concurrent games without degradation
- Automated tests run in CI/CD pipeline

#### 1.2 Server Reliability
**Estimated Time:** 1-2 weeks

- [ ] **Connection Management**
  - Implement heartbeat/ping-pong protocol
  - Detect disconnected clients within 30 seconds
  - Graceful handling of client disconnects
  - Reconnection with game state recovery (5-minute window)
  - Exponential backoff for reconnection attempts

- [ ] **Room Lifecycle Management**
  - Automatically clean up rooms after game completion
  - Remove abandoned rooms (all players disconnected > 10 minutes)
  - Persist active game state to disk/database
  - Implement room timeout configuration

- [ ] **Error Handling & Validation**
  - Validate all incoming messages against schema
  - Reject invalid moves with descriptive errors
  - Handle panics gracefully (catch_unwind)
  - Log all errors with context and timestamps

**Acceptance Criteria:**
- Zero crashes or panics under normal operation
- Clients can reconnect and resume games
- Abandoned rooms cleaned up within 15 minutes
- All error cases have tests and documentation

#### 1.3 Basic Security
**Estimated Time:** 3-5 days

- [ ] **Input Validation**
  - Sanitize room IDs (alphanumeric only, 4 chars)
  - Validate player names (length, character set)
  - Validate tile placement moves (rules enforcement)
  - Rate limit message frequency per connection

- [ ] **Connection Limits**
  - Max 8 players per room (enforce server-side)
  - Max 100 concurrent connections per IP
  - Max 10 rooms created per IP per hour
  - Implement IP-based rate limiting

- [ ] **Resource Protection**
  - Set maximum message size (1 MB)
  - Set connection timeout (30 seconds idle)
  - Implement graceful shutdown with drain period
  - Memory limits per game session

**Acceptance Criteria:**
- Server rejects all invalid inputs
- Rate limiting prevents spam/abuse
- Resource exhaustion attacks mitigated
- Security audit passes basic checklist

### Phase 2: Infrastructure & Deployment (1-2 weeks)

**Priority: HIGH** - Needed for public deployment

#### 2.1 CI/CD Pipeline
**Estimated Time:** 3-4 days

- [ ] **GitHub Actions Workflow**
  - Run tests on all PRs and commits to main
  - Build and test native client for macOS, Linux, Windows
  - Build WASM client and run WASM-specific tests
  - Build Docker images for server and client
  - Run clippy and rustfmt checks
  - Security scanning with cargo-audit

- [ ] **Automated Deployment**
  - Deploy server to staging on merge to develop
  - Deploy to production on release tags
  - Automated rollback on health check failure
  - Blue-green deployment strategy

**Acceptance Criteria:**
- All tests pass before merge
- Failed builds block deployment
- Zero-downtime deployments
- Rollback completes in < 5 minutes

#### 2.2 Production Environment Setup
**Estimated Time:** 3-5 days

**Server Hosting:**
- Deploy on cloud provider (AWS, GCP, DigitalOcean)
- Use managed container service (ECS, Cloud Run, Kubernetes)
- Configure auto-scaling (2-10 instances)
- Set up load balancer with health checks
- Enable HTTPS/WSS with Let's Encrypt

**Client Hosting:**
- Serve WASM client from CDN (Cloudflare, AWS CloudFront)
- Configure proper MIME types for .wasm files
- Enable gzip/brotli compression
- Set cache headers for static assets
- Configure CORS for WebSocket connections

**Monitoring & Observability:**
- Structured logging (JSON format)
- Log aggregation (CloudWatch, Stackdriver, Datadog)
- Metrics collection (Prometheus/Grafana)
- Alerting on error rates and latency
- Uptime monitoring (UptimeRobot, Pingdom)

**Acceptance Criteria:**
- 99.9% uptime SLA
- Average response time < 100ms
- Logs searchable and retained for 30 days
- Alerts trigger within 5 minutes of issues

#### 2.3 Domain & SSL
**Estimated Time:** 1 day

- [ ] Register domain name (e.g., tsurust.game)
- [ ] Configure DNS records (A, AAAA, CNAME)
- [ ] Set up SSL certificates (Let's Encrypt)
- [ ] Enable HTTPS redirect
- [ ] Configure WSS (secure WebSocket)

**Acceptance Criteria:**
- All traffic uses HTTPS/WSS
- SSL Labs rating A or higher
- No mixed content warnings

### Phase 3: User Experience & Polish (1 week)

**Priority: MEDIUM** - Improves usability and retention

#### 3.1 Error Handling & Feedback
**Estimated Time:** 2-3 days

- [ ] **Visual Error Messages**
  - Toast notifications for network errors
  - Modal dialogs for critical errors
  - Inline validation for lobby inputs
  - Retry buttons with exponential backoff

- [ ] **Loading States**
  - Spinner during room creation/join
  - Progress bar for game initialization
  - Skeleton UI while loading game state
  - Optimistic UI updates with rollback

- [ ] **Connection Status**
  - Indicator in UI (green/yellow/red)
  - Reconnecting message with countdown
  - Notification when connection restored
  - Offline mode with local play fallback

**Acceptance Criteria:**
- Users never see raw error messages
- All network operations have loading states
- Connection status always visible
- User satisfaction rating > 4/5

#### 3.2 Onboarding & Help
**Estimated Time:** 2-3 days

- [ ] **First-Time User Experience**
  - Welcome screen with game overview
  - Interactive tutorial (optional)
  - "How to Play" modal in main menu
  - Sample game button for quick start

- [ ] **In-Game Help**
  - Tooltips on hover for UI elements
  - Help button in top-right corner
  - Quick reference for tile rotation
  - FAQ page linked from menu

**Acceptance Criteria:**
- New users can start playing within 2 minutes
- Tutorial completion rate > 60%
- Help documentation clear and complete

#### 3.3 Mobile & Responsive Design
**Estimated Time:** 2-3 days

- [ ] **Mobile Web Support**
  - Touch-friendly UI (larger buttons)
  - Responsive layout for portrait/landscape
  - Virtual keyboard handling
  - Prevent zoom on double-tap

- [ ] **Progressive Web App (PWA)**
  - Web manifest for "Add to Home Screen"
  - Service worker for offline assets
  - App icons and splash screens
  - Push notifications for turn alerts (future)

**Acceptance Criteria:**
- Playable on mobile devices (iOS, Android)
- No horizontal scrolling required
- Touch gestures work intuitively
- PWA installable on mobile

### Phase 4: Advanced Features (Post-Launch)

**Priority: LOW** - Nice-to-have enhancements

#### 4.1 User Accounts & Persistence
- User registration and authentication (OAuth, email)
- Persistent game history and statistics
- Friend lists and private lobbies
- Player profiles and avatars

#### 4.2 Matchmaking
- Quick match (auto-join available rooms)
- ELO rating system
- Ranked and casual modes
- Tournament brackets

#### 4.3 Social Features
- In-game chat (with moderation)
- Spectator mode for ongoing games
- Replay sharing via URL
- Leaderboards and achievements

#### 4.4 Monetization (Optional)
- Cosmetic skins for tiles/boards
- Premium features (larger lobbies, custom rules)
- Ad-supported free tier
- One-time purchase or subscription model

---

## Resource Requirements

### Development Time Estimate
- **Phase 1 (Critical):** 2-3 weeks (1 developer)
- **Phase 2 (Infrastructure):** 1-2 weeks (1 developer)
- **Phase 3 (UX Polish):** 1 week (1 developer)
- **Total to MVP Launch:** 4-6 weeks

### Infrastructure Costs (Monthly)
- **Server Hosting:** $20-100/month (starts small, scales with users)
  - Digital Ocean Droplet ($12/month for 2GB RAM)
  - AWS ECS/Fargate ($20-50/month with auto-scaling)
- **CDN for Client:** $0-20/month (Cloudflare free tier, then paid)
- **Domain Name:** $10-15/year
- **SSL Certificate:** $0 (Let's Encrypt)
- **Monitoring/Logging:** $0-50/month (free tiers available)
- **Database (optional):** $0-20/month (if persistent storage needed)

**Total Estimated Cost:** $30-150/month

### Scaling Considerations
- **100 concurrent players:** Current setup sufficient
- **1,000 concurrent players:** Need horizontal scaling, load balancer ($100-200/month)
- **10,000+ concurrent players:** Multi-region deployment, database cluster, CDN ($500-1000/month)

---

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Server crashes under load | Medium | High | Load testing, monitoring, auto-restart |
| WebSocket connection issues | High | High | Reconnection logic, fallback to polling |
| WASM compatibility issues | Low | Medium | Browser testing, feature detection |
| Data loss on server restart | Medium | High | Persist game state, in-memory backup |
| Security vulnerabilities | Medium | High | Security audit, input validation, rate limiting |
| Performance degradation | Medium | Medium | Profiling, optimization, caching |

### Business Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Low user adoption | Medium | High | Marketing, social media, game communities |
| High infrastructure costs | Low | Medium | Start with minimal setup, scale gradually |
| Competitor launches first | Low | Medium | Focus on unique features, quality over speed |
| Negative user reviews | Medium | High | Beta testing, user feedback, rapid iteration |
| Legal issues (trademark, etc.) | Low | High | Legal review, trademark search, terms of service |

---

## Success Metrics

### Launch Metrics (First 30 Days)
- **User Acquisition:** 100+ unique users
- **Retention:** 30%+ return after 1 week
- **Uptime:** 99%+ server availability
- **Performance:** <200ms average latency
- **Errors:** <0.1% error rate

### Growth Metrics (3-6 Months)
- **Monthly Active Users:** 500-1000
- **Daily Active Users:** 100-200
- **Games Played:** 5000+
- **Average Session Length:** 15+ minutes
- **User Rating:** 4+ stars

---

## Decision Points

### Go/No-Go Criteria for Launch

**MUST HAVE (Blockers):**
- ✅ All critical tests passing
- ✅ Server handles 50+ concurrent games
- ✅ Reconnection logic working
- ✅ Basic security measures in place
- ✅ HTTPS/WSS enabled
- ✅ Error handling and user feedback
- ✅ Monitoring and alerting set up

**SHOULD HAVE (Important but not blockers):**
- CI/CD pipeline
- Mobile responsive design
- User onboarding/tutorial
- Performance optimization

**NICE TO HAVE (Post-launch):**
- User accounts
- Matchmaking
- Social features
- Monetization

---

## Next Steps

### Immediate Actions (This Week)
1. **Review and approve this proposal**
2. **Set up project board** (GitHub Projects, Trello, etc.)
3. **Prioritize Phase 1 tasks** and create detailed tickets
4. **Start integration test implementation**
5. **Begin server reliability improvements**

### Week 2-3: Phase 1 Execution
- Daily stand-ups to track progress
- Code reviews for all PRs
- Weekly demo of new features/fixes

### Week 4-5: Phase 2 Execution
- Set up production environment
- Deploy to staging
- Beta testing with small group

### Week 6: Launch Preparation
- Final testing and bug fixes
- Documentation and help content
- Marketing materials and announcements
- Soft launch (limited users)

### Week 7+: Public Launch
- Monitor metrics and user feedback
- Rapid response to issues
- Iterate based on user needs
- Plan Phase 3 and 4 features

---

## Appendix

### A. Technology Stack

**Backend:**
- Language: Rust 1.83+
- Framework: Tokio async runtime
- WebSocket: tungstenite + tokio-tungstenite
- Serialization: serde + serde_json

**Frontend:**
- Language: Rust 1.83+ (compiles to WASM)
- UI Framework: egui 0.29
- Build Tool: Trunk (WASM), cargo (native)
- Networking: ewebsock (WASM WebSocket)

**Infrastructure:**
- Containerization: Docker + Docker Compose
- Orchestration: (TBD - Kubernetes, ECS, or Cloud Run)
- Load Balancer: (TBD - nginx, AWS ALB, or cloud-native)
- CDN: Cloudflare or AWS CloudFront
- Monitoring: Prometheus + Grafana or cloud-native

### B. Useful Resources

**Rust Web Services:**
- [Tokio Documentation](https://tokio.rs/)
- [WebSocket Protocol](https://datatracker.ietf.org/doc/html/rfc6455)
- [Rust Async Book](https://rust-lang.github.io/async-book/)

**WASM Deployment:**
- [Trunk Documentation](https://trunkrs.dev/)
- [WASM Deployment Guide](https://rustwasm.github.io/docs/book/deployment.html)

**DevOps:**
- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)
- [Kubernetes Basics](https://kubernetes.io/docs/tutorials/kubernetes-basics/)

### C. Contact & Support

**Project Repository:** https://github.com/gallowstree/tsurust
**Documentation:** See README.md and CLAUDE.md in repository
**Issue Tracker:** GitHub Issues

---

**Prepared by:** Claude Sonnet 4.5
**Date:** 2026-01-14
**Version:** 1.0 - Initial Draft
