# Infrastructure Documentation

## DNS Configuration

### Domain Mappings

Current DNS configuration for `jbci-convex-dev.com` domain:

| Subdomain | Target IP | Service Port | SSL |
|-----------|-----------|--------------|-----|
| `dashboard.jbci-convex-dev.com` | `34.84.147.50` | 6791 | ✓ |
| `api.jbci-convex-dev.com` | `34.84.147.50` | 3210 | ✓ |
| `jbci-convex-dev.com` | `34.84.147.50` | 3211 | ✓ |

### GCE Instance Details

- **Instance Name:** `convex-backend-instance`
- **Zone:** `asia-northeast1-a`
- **Machine Type:** `e2-standard-2`
- **External IP:** `34.84.147.50` (ephemeral)
- **Status:** RUNNING
- **Network Tags:** `convex-server`

### DNS Changes History

#### 2025-07-30
- **dashboard.jbci-convex-dev.com**: `34.85.74.49` → `34.84.147.50`
- **api.jbci-convex-dev.com**: `34.85.74.49` → `34.84.147.50`

## nginx Configuration

### Upload Limits

**Updated:** 2025-07-30  
**Change:** Increased upload limit from 32MiB to 100MiB

#### Configuration Location
- **File:** `/etc/nginx/sites-available/convex`
- **Backup:** `/etc/nginx/sites-available/convex.backup`

#### Applied Settings
```nginx
client_max_body_size 100M;
```

Applied to all server blocks:
- API endpoint (`api.jbci-convex-dev.com`)
- Dashboard (`dashboard.jbci-convex-dev.com`)
- Main domain (`jbci-convex-dev.com`)
- IP-based access

### Service Endpoints

#### API Endpoint (WebSocket Support)
- **Domain:** `api.jbci-convex-dev.com`
- **Proxy:** `http://localhost:3210`
- **Features:** WebSocket upgrade support, custom timeout settings

#### Dashboard
- **Domain:** `dashboard.jbci-convex-dev.com`
- **Proxy:** `http://localhost:6791`

#### HTTP Actions
- **Domain:** `jbci-convex-dev.com`
- **Proxy:** `http://localhost:3211`

#### IP Access
- **IP:** `34.84.147.50`
- **API Path:** `/api/` → `http://localhost:3210/`
- **Default:** `/` → `http://localhost:6791` (Dashboard)

### Firewall Configuration

#### Applied Rules
- **Rule:** `allow-convex-ports`
- **Ports:** 3210, 3211, 6791, 80, 443
- **Source:** `0.0.0.0/0`
- **Target Tags:** `convex-server`

## SSL/TLS Configuration

### Let's Encrypt Certificates
- **Certificate Path:** `/etc/letsencrypt/live/jbci-convex-dev.com/fullchain.pem`
- **Private Key:** `/etc/letsencrypt/live/jbci-convex-dev.com/privkey.pem`
- **Managed by:** Certbot
- **Auto-renewal:** Enabled

### HTTPS Redirects
All HTTP traffic (port 80) automatically redirects to HTTPS (port 443).

## Domain Registration

### Cloud Domains Status
- **Domain:** `jbci-convex-dev.com`
- **Status:** ACTIVE
- **Renewal:** AUTOMATIC_RENEWAL
- **Expires:** 2026-07-09T01:19:59Z
- **DNS Provider:** Cloud DNS

### Billing Account
- **Account ID:** `0195AF-11AB54-9E5E07`
- **Name:** 請求先アカウント
- **Status:** OPEN (Active)

## Maintenance Notes

### Static IP Recommendation
Currently using ephemeral IP (`34.84.147.50`). Consider:
- Reserve static IP to prevent changes on instance restart
- Update DNS automation if static IP is implemented

### Load Balancer Consideration
For production scaling:
- Implement Google Cloud Load Balancer
- Enable autoscaling with instance groups
- Configure health checks for high availability

### Monitoring
- Monitor SSL certificate auto-renewal
- Track domain expiration dates
- Monitor DNS propagation for changes

---
*Last Updated: 2025-07-30*