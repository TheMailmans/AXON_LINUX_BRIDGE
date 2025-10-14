#!/bin/bash
# Direct test of Calculator button clicks
# This will help us find the actual button positions

set -e

echo "=== Calculator Button Position Test ==="
echo ""
echo "This script will:"
echo "  1. Close any existing Calculator"
echo "  2. Launch Calculator"
echo "  3. Find button positions"
echo "  4. Click buttons 1, 2, 3 using xdotool directly"
echo ""
echo "WATCH YOUR SCREEN - You should see clicks happen!"
echo ""

# Step 1: Kill any existing Calculator
echo "1. Closing existing Calculator instances..."
pkill -f gnome-calculator || true
sleep 1

# Step 2: Launch Calculator
echo "2. Launching Calculator..."
gio launch /usr/share/applications/org.gnome.Calculator.desktop &
sleep 3

# Step 3: Get Calculator window
echo "3. Finding Calculator window..."
CALC_WIN=$(xdotool search --class "gnome-calculator" | head -1)
if [ -z "$CALC_WIN" ]; then
    echo "ERROR: Calculator window not found!"
    exit 1
fi
echo "   Window ID: $CALC_WIN"

# Step 4: Activate window
echo "4. Activating Calculator window..."
xdotool windowactivate "$CALC_WIN"
sleep 0.5

# Step 5: Get window position and size
echo "5. Getting window geometry..."
GEOM=$(xdotool getwindowgeometry "$CALC_WIN")
echo "$GEOM"

# Extract position
POS_LINE=$(echo "$GEOM" | grep "Position:")
X_WIN=$(echo "$POS_LINE" | awk '{print $2}' | cut -d',' -f1)
Y_WIN=$(echo "$POS_LINE" | awk '{print $2}' | cut -d',' -f2)

# Extract size
SIZE_LINE=$(echo "$GEOM" | grep "Geometry:")
WIDTH=$(echo "$SIZE_LINE" | awk '{print $2}' | cut -d'x' -f1)
HEIGHT=$(echo "$SIZE_LINE" | awk '{print $2}' | cut -d'x' -f2)

echo "   Window at: X=$X_WIN, Y=$Y_WIN"
echo "   Window size: ${WIDTH}x${HEIGHT}"

# Step 6: Calculate button positions
# GNOME Calculator layout (typical):
# - Display at top
# - Numeric buttons in grid starting around 2/3 down
# - Button "1" is typically in bottom-left of number grid
# - Buttons are ~60px wide with ~5px spacing

# Calculate relative to window
BUTTON_START_Y=$((Y_WIN + HEIGHT - 180))  # Buttons start ~180px from bottom
BUTTON_1_X=$((X_WIN + 40))                # Button 1 at left
BUTTON_2_X=$((X_WIN + 110))               # Button 2 in middle
BUTTON_3_X=$((X_WIN + 180))               # Button 3 at right
BUTTON_Y=$BUTTON_START_Y

echo ""
echo "6. Calculated button positions:"
echo "   Button 1: ($BUTTON_1_X, $BUTTON_Y)"
echo "   Button 2: ($BUTTON_2_X, $BUTTON_Y)"
echo "   Button 3: ($BUTTON_3_X, $BUTTON_Y)"
echo ""

# Step 7: Test clicks with visual delays
echo "=== Starting Click Test ==="
echo ""
echo "⏳ Watch the Calculator display..."
sleep 2

echo "7. Clicking button '1'..."
xdotool mousemove $BUTTON_1_X $BUTTON_Y
sleep 0.5
xdotool click 1
sleep 1

echo "8. Clicking button '2'..."
xdotool mousemove $BUTTON_2_X $BUTTON_Y
sleep 0.5
xdotool click 1
sleep 1

echo "9. Clicking button '3'..."
xdotool mousemove $BUTTON_3_X $BUTTON_Y
sleep 0.5
xdotool click 1
sleep 1

echo ""
echo "=== Test Complete ==="
echo ""
echo "📊 Results:"
echo "   ✅ Commands executed successfully"
echo ""
echo "🔍 Visual Check:"
echo "   Does the Calculator display show '123'?"
echo ""
echo "   YES → xdotool clicks are working correctly"
echo "   NO  → Need to adjust button coordinates"
echo ""
echo "💡 If '123' appears, the Bridge should work identically!"
echo "   (Bridge uses the same xdotool commands)"
