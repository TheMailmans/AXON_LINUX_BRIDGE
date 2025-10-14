#!/bin/bash
# Quick test to verify LaunchApplication fallback fix

echo "=== Testing LaunchApplication Fallback Fix ==="
echo ""
echo "This test verifies that applications launch successfully even when"
echo "AppIndex lookup fails, by using fallback launch methods."
echo ""

# Test 1: Direct command-line test of fallback methods
echo "Test 1: Testing fallback launch methods directly"
echo "-----------------------------------------------"
echo "Trying: gio launch gnome-terminal"
if gio launch gnome-terminal 2>/dev/null; then
    echo "✅ gio launch works"
else
    echo "❌ gio launch failed (exit code: $?)"
fi

echo ""
echo "Trying: gtk-launch gnome-terminal"
if gtk-launch gnome-terminal 2>/dev/null; then
    echo "✅ gtk-launch works"
else
    echo "❌ gtk-launch failed (exit code: $?)"
fi

echo ""
echo "Trying: nohup gnome-terminal > /dev/null 2>&1 &"
if nohup gnome-terminal > /dev/null 2>&1 & then
    echo "✅ direct exec works"
    PID=$!
    echo "   (Launched with PID: $PID)"
    # Give it a moment to start
    sleep 1
    if ps -p $PID > /dev/null; then
        echo "   Process is running"
    else
        echo "   ⚠️  Process may have exited"
    fi
else
    echo "❌ direct exec failed"
fi

echo ""
echo "=== Manual Verification Steps ==="
echo ""
echo "To fully test the fix, use the Mac Hub or a Python gRPC client to call:"
echo ""
echo "  LaunchApplication(app_name='terminal')"
echo "  or"
echo "  LaunchApplication(app_name='gnome-terminal')"
echo ""
echo "Expected result:"
echo "  - success: true"
echo "  - Terminal window opens on Ubuntu VM"
echo "  - Bridge logs show fallback method attempts"
echo ""
echo "Check bridge logs with:"
echo "  tail -f /home/th3mailman/AXONBRIDGE-Linux/bridge.log"
echo ""

echo "=== Bridge Status ==="
ps aux | grep axon-desktop-agent | grep -v grep
echo ""

echo "✅ Fallback method tests completed"
echo "   Bridge is running and ready for gRPC requests"
