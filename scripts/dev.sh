#!/bin/bash
# Development script for Nebula Code

set -e

echo "Starting Nebula Code development environment..."

# Function to start a specific app
start_app() {
    local app=$1
    case $app in
        desktop)
            echo "Starting desktop app..."
            cd apps/desktop
            pnpm dev
            ;;
        cli)
            echo "Starting CLI development..."
            cd apps/cli
            cargo watch -x run
            ;;
        marketplace)
            echo "Starting marketplace app..."
            cd apps/marketplace
            pnpm dev
            ;;
        *)
            echo "Unknown app: $app"
            echo "Available apps: desktop, cli, marketplace"
            exit 1
            ;;
    esac
}

# If no arguments, start all apps
if [ $# -eq 0 ]; then
    echo "No app specified. Starting all development servers..."
    echo "Use Ctrl+C to stop individual servers."
    
    # Start marketplace in background
    cd apps/marketplace && pnpm dev &
    MARKETPLACE_PID=$!
    
    # Start desktop in background
    cd apps/desktop && pnpm dev &
    DESKTOP_PID=$!
    
    # Wait for processes
    wait
else
    start_app "$1"
fi
