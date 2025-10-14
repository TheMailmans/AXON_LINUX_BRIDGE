#!/bin/bash
# Test script to verify clicks are working visibly

set -e

echo "=== Observable Click Test ==="
echo ""

# Step 1: Launch Calculator
echo "1. Launching Calculator..."
gio launch /usr/share/applications/org.gnome.Calculator.desktop
sleep 2

# Step 2: Get Calculator window ID
echo "2. Finding Calculator window..."
CALC_WINDOW=$(wmctrl -l | grep -i calculator | head -1 | awk '{print $1}')
if [ -z "$CALC_WINDOW" ]; then
    echo "ERROR: Calculator window not found!"
    exit 1
fi
echo "   Found window: $CALC_WINDOW"

# Step 3: Activate Calculator window
echo "3. Activating Calculator window..."
wmctrl -i -a "$CALC_WINDOW"
sleep 1

# Step 4: Get window geometry
echo "4. Getting window position..."
GEOM=$(xdotool getwindowgeometry "$CALC_WINDOW" | grep Position | awk '{print $2}')
echo "   Window position: $GEOM"

# Step 5: Calculate click coordinates (center of calculator button area)
X=$(echo $GEOM | cut -d',' -f1)
Y=$(echo $GEOM | cut -d',' -f2)
CLICK_X=$((X + 100))
CLICK_Y=$((Y + 150))

echo "5. Will click at: ($CLICK_X, $CLICK_Y)"
echo ""
echo "WATCH THE CALCULATOR NOW - clicking in 2 seconds..."
sleep 2

# Step 6: Perform visible click
echo "6. CLICKING NOW!"
xdotool mousemove "$CLICK_X" "$CLICK_Y"
sleep 0.5
xdotool click 1
echo "   Click executed!"

echo ""
echo "=== Test Complete ==="
echo "Did you see the mouse move and click on the Calculator?"
