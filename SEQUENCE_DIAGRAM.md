# Solana Trading Bot - Sequence Diagram Documentation

## Overview

This document describes the sequence of interactions between the various modules and external services in the Solana trading bot application.

## System Architecture

```mermaid
graph TB
    subgraph "Core Application"
        M[main.rs]
        C[config.rs]
        W[wallet.rs]
        T[trading.rs]
        J[jupiter.rs]
        F[firestore.rs]
        L[line_bot.rs]
    end
    
    subgraph "External Services"
        SOL[Solana Blockchain]
        JUP[Jupiter API]
        FS[Google Firestore]
        LINE[LINE Messaging API]
    end
    
    M --> C
    M --> W
    M --> T
    M --> F
    M --> L
    T --> J
    T --> F
    J --> W
    W --> SOL
    J --> SOL
    J --> JUP
    F --> FS
    L --> LINE
```

## Sequence Diagrams

### 1. Application Startup Sequence

```mermaid
sequenceDiagram
    participant main as main.rs
    participant config as config.rs
    participant wallet as wallet.rs
    participant line as line_bot.rs
    participant firestore as firestore.rs
    participant trading as trading.rs
    participant ext_fs as Firestore DB
    participant ext_line as LINE API

    main->>config: load_from_env()
    config-->>main: Config struct
    
    main->>wallet: Wallet::new(private_key)
    wallet-->>main: Wallet instance
    
    main->>line: Client::new(channel_token)
    line-->>main: LINE Client
    
    main->>firestore: get_client(service_account)
    firestore->>ext_fs: authenticate
    ext_fs-->>firestore: connection
    firestore-->>main: Firestore Client
    
    main->>trading: TradingState::new()
    main->>trading: load_state_from_firestore()
    trading->>firestore: get_latest_trading_state()
    firestore->>ext_fs: query trading_state
    ext_fs-->>firestore: state data
    firestore-->>trading: position, profits, counts
    trading-->>main: TradingState initialized
    
    main->>line: send_message("Bot started")
    line->>ext_line: POST /message/push
    ext_line-->>line: 200 OK
```

### 2. Trading Loop Sequence (Hourly Execution)

```mermaid
sequenceDiagram
    participant main as main.rs
    participant trading as trading.rs
    participant jupiter as jupiter.rs
    participant wallet as wallet.rs
    participant firestore as firestore.rs
    participant line as line_bot.rs
    participant sol as Solana RPC
    participant jup_api as Jupiter API
    participant fs_db as Firestore DB
    participant line_api as LINE API

    loop Every Hour
        main->>trading: check_and_trade()
        
        Note over trading: Get current prices
        trading->>jupiter: get_price(sol_usdc_pair)
        jupiter->>jup_api: GET /quote?inputMint=SOL&outputMint=USDC
        jup_api-->>jupiter: price quote
        jupiter-->>trading: current_price
        
        Note over trading: Store price history
        trading->>firestore: store_price(timestamp, price)
        firestore->>fs_db: insert price_history
        fs_db-->>firestore: success
        
        Note over trading: Analyze price trends
        trading->>firestore: get_price_trend(1h, 6h, 24h)
        firestore->>fs_db: query price_history
        fs_db-->>firestore: historical prices
        firestore-->>trading: trend percentages
        
        Note over trading: Evaluate trading signals
        alt Buy Signal (price drop > threshold)
            trading->>jupiter: execute_swap(USDC→SOL)
            jupiter->>jup_api: POST /swap
            jup_api-->>jupiter: swap_transaction
            jupiter->>wallet: sign_transaction()
            wallet-->>jupiter: signed_tx
            jupiter->>sol: send_transaction()
            sol-->>jupiter: tx_signature
            jupiter-->>trading: swap_result
            trading->>trading: update_position("long")
        else Sell Signal (price rise > threshold)
            trading->>jupiter: execute_swap(SOL→USDC)
            jupiter->>jup_api: POST /swap
            jup_api-->>jupiter: swap_transaction
            jupiter->>wallet: sign_transaction()
            wallet-->>jupiter: signed_tx
            jupiter->>sol: send_transaction()
            sol-->>jupiter: tx_signature
            jupiter-->>trading: swap_result
            trading->>trading: update_position("none")
            trading->>trading: calculate_profit()
        end
        
        Note over trading: Store trading session
        trading->>firestore: store_trading_session()
        firestore->>fs_db: insert trading_sessions
        fs_db-->>firestore: success
        
        trading-->>main: trade_result
        
        Note over main: Send notifications
        main->>line: send_trade_notification()
        line->>line_api: POST /message/push
        line_api-->>line: 200 OK
    end
```

### 3. API Request Handling Sequence

```mermaid
sequenceDiagram
    participant client as HTTP Client
    participant main as main.rs
    participant config as config.rs
    participant firestore as firestore.rs
    participant fs_db as Firestore DB

    Note over client,main: GET /api/prices
    client->>main: HTTP GET /api/prices?hours=24
    main->>config: get_config()
    config-->>main: config
    main->>firestore: get_price_history(24h)
    firestore->>fs_db: query price_history
    fs_db-->>firestore: price records
    firestore-->>main: Vec<PricePoint>
    main-->>client: JSON response
    
    Note over client,main: GET /api/performance
    client->>main: HTTP GET /api/performance
    main->>main: get_current_state()
    main-->>client: JSON {position, profit, trades}
    
    Note over client,main: GET /api/sessions
    client->>main: HTTP GET /api/sessions?days=7
    main->>firestore: get_trading_sessions(7d)
    firestore->>fs_db: query trading_sessions
    fs_db-->>firestore: session records
    firestore-->>main: Vec<TradingSession>
    main-->>client: JSON response
```

### 4. Error Handling and Recovery Sequence

```mermaid
sequenceDiagram
    participant main as main.rs
    participant trading as trading.rs
    participant jupiter as jupiter.rs
    participant line as line_bot.rs
    participant sol as Solana RPC
    participant line_api as LINE API

    main->>trading: check_and_trade()
    trading->>jupiter: execute_swap()
    jupiter->>sol: send_transaction()
    sol-->>jupiter: Error: insufficient funds
    jupiter-->>trading: Err(InsufficientFunds)
    trading-->>main: Err(TradingError)
    
    Note over main: Error handling
    main->>line: send_error_notification()
    line->>line_api: POST /message/push
    line_api-->>line: 200 OK
    
    Note over main: Continue operation
    main->>main: log_error()
    main->>main: wait_for_next_cycle()
```

## Module Responsibilities

### config.rs
- Loads environment variables
- Provides centralized configuration
- Validates required settings

### wallet.rs
- Manages Solana keypair
- Signs transactions
- Queries blockchain balances

### trading.rs
- Implements trading strategy
- Maintains position state
- Calculates profits/losses
- Triggers buy/sell decisions

### jupiter.rs
- Interfaces with Jupiter DEX aggregator
- Gets optimal swap routes
- Executes token swaps
- Handles transaction building

### firestore.rs
- Manages database connections
- Stores price history
- Records trading sessions
- Provides data queries
- Implements retry logic

### line_bot.rs
- Sends trade notifications
- Reports errors
- Notifies system status

### main.rs
- Orchestrates all modules
- Runs hourly trading loop
- Provides HTTP API
- Handles graceful shutdown

## External Service Dependencies

1. **Solana Blockchain (RPC)**
   - Used for: Transaction submission, balance queries
   - Accessed by: wallet.rs, jupiter.rs

2. **Jupiter API**
   - Used for: DEX aggregation, swap routing
   - Endpoint: https://quote-api.jup.ag
   - Accessed by: jupiter.rs

3. **Google Firestore**
   - Used for: Persistent storage
   - Collections: price_history, trading_sessions, trading_state
   - Accessed by: firestore.rs

4. **LINE Messaging API**
   - Used for: Push notifications
   - Endpoint: https://api.line.me
   - Accessed by: line_bot.rs

## Data Flow Summary

1. **Configuration Flow**: Environment → config.rs → all modules
2. **Price Data Flow**: Jupiter API → jupiter.rs → trading.rs → firestore.rs → Firestore DB
3. **Trade Execution Flow**: trading.rs → jupiter.rs → wallet.rs → Solana blockchain
4. **Notification Flow**: trading.rs → main.rs → line_bot.rs → LINE API
5. **API Query Flow**: HTTP client → main.rs → firestore.rs → Firestore DB → HTTP response

## Key Design Decisions

1. **Modular Architecture**: Each module has a single responsibility
2. **Async Operations**: All I/O operations use async/await for performance
3. **Error Propagation**: Errors bubble up to main.rs for centralized handling
4. **State Persistence**: Trading state survives restarts via Firestore
5. **External Service Abstraction**: External APIs are wrapped in dedicated modules