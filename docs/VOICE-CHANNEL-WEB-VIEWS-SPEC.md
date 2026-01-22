# LinguaBridge Voice Channel Web Views with x402 Payment Integration

## Specification Document

### Version

1.0.0

### Date

January 18, 2026

### Authors

LinguaBridge Development Team

## 1. Introduction

### 1.1 Purpose

This specification outlines the design and implementation requirements for a single-instance, channel-specific web interface for real-time voice translation display, with x402 payment protocol integration for access control. The system replaces the current session-based access model with public URLs per voice channel, monetized through one-time payments per Discord server using cosmwasm contracts on the Cosmos ecosystem.

### 1.2 Scope

**In Scope:**

- Single public URL per voice channel for real-time translation display
- x402 payment protocol integration using cosmwasm contracts
- One-time payment unlocks entire Discord server for web access
- Real-time WebSocket broadcasting of voice transcriptions
- Reduced backend computation through single instances per channel

**Out of Scope:**

- Text channel translation web views (existing functionality preserved)
- Voice inference pipeline modifications
- Discord bot core functionality changes
- Mobile application development

### 1.3 Definitions

- **x402 Protocol**: HTTP-native payment protocol using HTTP 402 status code for programmatic payments
- **Cosmwasm**: Smart contract platform for Cosmos blockchains
- **Voice Channel Instance**: Single web client instance serving all users viewing a specific Discord voice channel
- **Server Unlock**: Payment status that enables web access for all voice channels in a Discord server

### 1.4 References

- [x402 Protocol Specification](https://x402.org)
- [Cosmwasm Documentation](https://docs.cosmwasm.com)
- [Discord Developer Documentation](https://discord.com/developers/docs)
- [LinguaBridge Voice Pipeline Documentation](./docs/voice-inference-pipeline.md)

## 2. Requirements

### 2.1 Functional Requirements

#### 2.1.1 Web Interface

- **VC-001**: System shall provide public URLs in format `/voice/{guild_id}/{channel_id}`
- **VC-002**: URLs shall be accessible without authentication if server is unlocked via payment
- **VC-003**: Interface shall display real-time transcriptions and translations from voice channel
- **VC-004**: WebSocket connections shall provide live updates without page refresh
- **VC-005**: Interface shall support multiple concurrent viewers per channel

#### 2.1.2 Payment Integration

- **PAY-001**: System shall use x402 protocol for payment enforcement
- **PAY-002**: HTTP 402 responses shall be returned for unpaid server access attempts
- **PAY-003**: Payments shall be verified via cosmwasm contract queries
- **PAY-004**: One-time payments shall unlock entire Discord server for web access
- **PAY-005**: Payment status shall be cached to reduce contract query frequency

#### 2.1.3 Voice Broadcasting

- **VB-001**: Voice inference results shall be broadcast to all active channel viewers
- **VB-002**: Broadcasting shall be channel-specific rather than session-specific
- **VB-003**: Single voice manager instance shall serve all channels globally
- **VB-004**: Voice results shall be forwarded from inference service to web broadcast manager

#### 2.1.4 Discord Integration

- **DC-001**: Bot commands shall provide public URLs for voice channels
- **DC-002**: Server unlock status shall be queryable via Discord commands
- **DC-003**: Payment initiation shall be available through Discord bot interface

### 2.2 Non-Functional Requirements

#### 2.2.1 Performance

- **PERF-001**: Web interface shall load within 3 seconds
- **PERF-002**: Real-time updates shall have <500ms latency from voice to display
- **PERF-003**: System shall support up to 100 concurrent viewers per voice channel
- **PERF-004**: Payment status queries shall be cached for 5 minutes

#### 2.2.2 Security

- **SEC-001**: Payment verification shall validate on-chain transactions
- **SEC-002**: Web access shall only be granted after confirmed payment
- **SEC-003**: URLs shall be public but content protected by payment status
- **SEC-004**: Contract queries shall use authenticated gRPC connections

#### 2.2.3 Scalability

- **SCALE-001**: Single instance per voice channel shall reduce computation overhead
- **SCALE-002**: Channel-based broadcasting shall scale better than session-based
- **SCALE-003**: Payment caching shall reduce contract query load

### 2.3 Technical Requirements

#### 2.3.1 Blockchain Integration

- **TECH-001**: Cosmwasm contract shall be deployed on Cosmos ecosystem chain
- **TECH-002**: Contract shall support guild_id to payment_status mappings
- **TECH-003**: Contract queries shall be available via gRPC endpoints
- **TECH-004**: Multiple stablecoin support (axlUSDC, USTC, etc.)

#### 2.3.2 Web Architecture

- **TECH-005**: Rust Axum web server with WebSocket support
- **TECH-006**: Channel-specific broadcast channels using Tokio broadcast
- **TECH-007**: x402 middleware for payment enforcement
- **TECH-008**: CORS support for cross-origin access

## 3. Architecture

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Discord Voice Channel                    │
│                    (Users speaking)                         │
└─────────────────────┬───────────────────────────────────────┘
                      │ Opus audio
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                Voice Inference Pipeline                     │
│  STT (Whisper) → Translation (Gemma) → TTS (CosyVoice)     │
└─────────────────────┬───────────────────────────────────────┘
                      │ Results
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                    Voice Broadcast Manager                   │
│                (Global voice result routing)                │
└─────────────────────┬───────────────────────────────────────┘
                      │ Channel-specific
                      ▼ broadcasts
┌─────────────────────────────────────────────────────────────┐
│                     Web Interface Layer                      │
│  ┌─────────────────────────────────┬─────────────────────┐  │
│  │   x402 Payment Middleware       │  WebSocket Server   │  │
│  │   (Checks server unlock)        │  (Live updates)     │  │
│  └─────────────────────────────────┴─────────────────────┘  │
└─────────────────────┬───────────────────────────────────────┘
                      │ HTTP/WebSocket
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                   User Web Browsers                         │
│              (Multiple viewers per channel)                 │
└─────────────────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                Cosmwasm Payment Contract                    │
│            (Server unlock status tracking)                  │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Component Descriptions

#### 3.2.1 Voice Broadcast Manager

- **Purpose**: Routes voice inference results to appropriate web broadcast channels
- **Input**: VoiceInferenceResponse from voice inference pipeline
- **Output**: Channel-specific broadcasts to web clients
- **Implementation**: Tokio broadcast channels keyed by (guild_id, channel_id)

#### 3.2.2 x402 Payment Middleware

- **Purpose**: Enforces payment requirements for web access
- **Input**: HTTP requests to voice channel endpoints
- **Output**: HTTP 402 for unpaid access, normal response for paid access
- **Implementation**: Axum middleware checking contract payment status

#### 3.2.3 WebSocket Server

- **Purpose**: Provides real-time updates to web clients
- **Input**: Broadcast messages from Voice Broadcast Manager
- **Output**: JSON messages to connected web clients
- **Implementation**: Axum WebSocket upgrade with tokio-tungstenite

#### 3.2.4 Cosmwasm Contract

- **Purpose**: Tracks payment status and server unlock state
- **Functions**:
  - `unlock_server(guild_id)` - Mark server as paid
  - `query_server_status(guild_id)` - Check payment status
  - `get_payment_records(guild_id)` - Retrieve payment history

### 3.3 Data Flow

#### 3.3.1 Voice to Web Display Flow

1. User speaks in Discord voice channel
2. Audio captured by Songbird voice handler
3. Audio sent to voice inference service (Python)
4. STT → Translation → Results returned to Rust bot
5. Voice Broadcast Manager receives results
6. Results broadcast to all viewers of that channel
7. Web clients receive real-time updates via WebSocket

#### 3.3.2 Payment Verification Flow

1. User accesses `/voice/{guild_id}/{channel_id}`
2. x402 middleware checks payment status
3. If unpaid: Return HTTP 402 with payment instructions
4. User pays via Cosmos wallet to contract
5. Contract updates server unlock status
6. User retries request with payment proof
7. Middleware validates payment and grants access

## 4. API Design

### 4.1 Web Endpoints

#### 4.1.1 Voice Channel View

```
GET /voice/{guild_id}/{channel_id}
```

**Purpose**: Serve the web interface for viewing voice channel translations

**Parameters**:

- `guild_id`: Discord guild/server ID
- `channel_id`: Discord voice channel ID

**Responses**:

- `200 OK`: HTML page if server is unlocked
- `402 Payment Required`: x402 payment response if server unpaid
- `404 Not Found`: Invalid guild/channel combination

**Headers**:

- `X-Payment-Required: true` (when 402 returned)
- `X-Payment-Contract: <contract_address>` (Cosmos contract address)
- `X-Payment-Amount: <amount>` (Required payment in stablecoin)

#### 4.1.2 Voice Channel WebSocket

```
GET /voice/{guild_id}/{channel_id}/ws
```

**Purpose**: Establish WebSocket connection for real-time updates

**Parameters**: Same as above

**Protocol**:

- **Connection**: Standard WebSocket upgrade
- **Authentication**: Payment status checked on initial HTTP request
- **Messages**: JSON objects with voice transcription data
- **Heartbeat**: Client sends ping every 30s, server responds with pong

**Message Format**:

```json
{
  "type": "voice_transcription",
  "guild_id": "123456789",
  "channel_id": "987654321", 
  "user_id": "111222333",
  "username": "JohnDoe",
  "original_text": "Hello world",
  "translated_text": "Hola mundo",
  "source_lang": "en",
  "target_lang": "es",
  "latency_ms": 450,
  "timestamp": 1640995200000
}
```

### 4.2 Discord Bot Commands

#### 4.2.1 Get Voice Channel URL

```
/voice url [channel]
```

**Purpose**: Get the public web URL for a voice channel

**Parameters**:

- `channel`: Optional voice channel mention (defaults to user's current channel)

**Response**: Discord embed with public URL and usage instructions

#### 4.2.2 Server Unlock Status

```
/server status
```

**Purpose**: Check payment/unlock status for the current server

**Response**: Payment status and unlock information

#### 4.2.3 Initiate Server Unlock

```
/server unlock
```

**Purpose**: Start the payment process to unlock web access for the server

**Response**: Payment instructions with Cosmos address and amount

### 4.3 Cosmwasm Contract Interface

#### 4.3.1 Query Server Status

```rust
#[derive(Serialize, Deserialize)]
pub struct ServerStatusQuery {
    pub server_status: {
        pub guild_id: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct ServerStatusResponse {
    pub unlocked: bool,
    pub unlocked_at: Option<u64>, // timestamp
    pub payment_tx: Option<String>,
}
```

#### 4.3.2 Unlock Server

```rust
#[derive(Serialize, Deserialize)]
pub struct UnlockServerMsg {
    pub unlock_server: {
        pub guild_id: String,
    },
}
```

## 5. Database Schema

### 5.1 Server Payments Table

```sql
CREATE TABLE server_payments (
    guild_id TEXT PRIMARY KEY,
    unlocked BOOLEAN NOT NULL DEFAULT FALSE,
    payment_tx_hash TEXT,
    unlocked_at DATETIME,
    expires_at DATETIME, -- For future time-based unlocks
    last_checked DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 5.2 Payment Records Table

```sql
CREATE TABLE payment_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    user_id TEXT, -- Discord user who initiated payment
    cosmos_tx_hash TEXT NOT NULL,
    amount TEXT NOT NULL, -- e.g., "1000000uaxlusdc"
    denom TEXT NOT NULL, -- stablecoin denomination
    status TEXT NOT NULL, -- 'pending', 'confirmed', 'failed'
    confirmed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_guild_status (guild_id, status),
    INDEX idx_tx_hash (cosmos_tx_hash)
);
```

### 5.3 Active Voice Channels Table

```sql
CREATE TABLE active_voice_channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_activity DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(guild_id, channel_id)
);
```

## 6. Payment Flow

### 6.1 User Journey

#### 6.1.1 Discovery

1. User joins Discord voice channel where bot is active
2. User runs `/voice url` to get web URL
3. User attempts to access URL in browser

#### 6.1.2 Payment Process

1. Browser receives HTTP 402 response with payment details
2. User opens Cosmos wallet (Keplr, Leap, etc.)
3. User sends payment to contract address with guild_id in memo
4. Contract executes `unlock_server` function
5. User refreshes browser or auto-retry occurs

#### 6.1.3 Access Granted

1. Middleware queries contract and finds server unlocked
2. Payment status cached for 5 minutes
3. HTML page served with WebSocket connection
4. Real-time transcriptions begin displaying

### 6.2 Contract Interaction

#### 6.2.1 Payment Transaction

```json
{
  "type": "cosmos-sdk/MsgExecuteContract",
  "value": {
    "sender": "cosmos1...",
    "contract": "cosmos1contractaddress...",
    "msg": {
      "unlock_server": {
        "guild_id": "123456789"
      }
    },
    "funds": [
      {
        "denom": "uaxlusdc",
        "amount": "5000000"
      }
    ]
  }
}
```

#### 6.2.2 Status Verification

- Web middleware queries contract via gRPC
- Response indicates unlock status and timestamp
- Status cached locally to reduce query frequency

## 7. Implementation Plan

### 7.1 Phase 1: Infrastructure Setup

**Duration**: 2 weeks

1. **Cosmwasm Contract Development**
   - Design contract interface for server unlocks
   - Implement payment verification logic
   - Deploy to testnet for development
   - Create query functions for web middleware

2. **Database Schema Updates**
   - Create payment and unlock status tables
   - Add migration scripts
   - Update existing models

3. **x402 Middleware Implementation**
   - HTTP 402 response handling
   - Cosmos transaction validation
   - Payment proof verification

### 7.2 Phase 2: Web Architecture Redesign

**Duration**: 2 weeks

1. **New Route Structure**
   - Implement `/voice/{guild_id}/{channel_id}` endpoints
   - Remove session-based authentication
   - Add payment status checking

2. **Broadcasting System Redesign**
   - Implement channel-based broadcasting
   - Create global voice result forwarding
   - Update WebSocket handling

3. **Voice Manager Global Storage**
   - Store VoiceManager in bot Data struct
   - Implement cross-command voice management
   - Background result processing task

### 7.3 Phase 3: Discord Integration

**Duration**: 1 week

1. **New Bot Commands**
   - `/voice url` command implementation
   - `/server unlock` and `/server status` commands
   - Payment instruction formatting

2. **Command Updates**
   - Modify existing voice commands to use global manager
   - Update help text and documentation

### 7.4 Phase 4: Testing and Deployment

**Duration**: 2 weeks

1. **Integration Testing**
   - End-to-end payment flow testing
   - Voice broadcasting verification
   - WebSocket real-time updates testing

2. **Performance Optimization**
   - Payment status caching implementation
   - Broadcast channel optimization
   - Load testing with multiple viewers

3. **Production Deployment**
   - Mainnet contract deployment
   - Configuration updates
   - Monitoring and logging setup

## 8. Security Considerations

### 8.1 Payment Security

- All payment transactions validated on-chain
- Contract addresses verified before queries
- Payment proofs checked for authenticity
- No private keys stored in web services

### 8.2 Access Control

- Public URLs but payment-gated content
- Server-level unlock (not per-user)
- No session tokens or cookies required
- CORS properly configured

### 8.3 Data Protection

- Voice transcriptions displayed in real-time
- No persistent storage of audio data
- User IDs and usernames handled appropriately
- GDPR compliance for EU users

## 9. Monitoring and Observability

### 9.1 Metrics

- Payment success/failure rates
- Voice channel active viewer counts
- WebSocket connection durations
- Contract query response times

### 9.2 Logging

- Payment transaction logs
- Voice broadcasting events
- Web access attempts and failures
- Contract query successes/failures

### 9.3 Alerts

- Contract connectivity issues
- Payment verification failures
- High latency in voice broadcasting
- Unusual access patterns

## 10. Future Enhancements

### 10.1 Advanced Features

- Time-based unlocks (subscription model)
- Per-channel pricing tiers
- Payment refunds for unused access
- Multi-language support for payment instructions

### 10.2 Scalability Improvements

- Regional contract deployments
- Payment status CDN caching
- Voice result batching for high-traffic channels
- Mobile-optimized payment flows

### 10.3 Analytics

- Usage analytics for voice channels
- Payment conversion tracking
- User engagement metrics
- Revenue reporting

## 11. Conclusion

This specification provides a comprehensive design for implementing single-instance voice channel web views with x402 payment integration. The approach reduces backend computation overhead while establishing a monetization model through one-time server unlocks.

The implementation prioritizes:

- **Performance**: Single instances and channel-based broadcasting
- **Security**: On-chain payment verification and proper access controls
- **Scalability**: Efficient caching and broadcasting mechanisms
- **User Experience**: Simple payment flows and real-time updates

The phased approach ensures incremental development with clear milestones and testing checkpoints. All components integrate with the existing LinguaBridge architecture while introducing the new payment and web access model.
