"""
AxonHub Text-First Agent
Connects to AxonHub Core via Ubuntu bridge to execute desktop actions.
"""

import base64
import grpc
import json
import time
from typing import Dict, Any, List, Optional

# You'll need to generate these from the proto file:
# python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. agent.proto
try:
    import agent_pb2
    import agent_pb2_grpc
except ImportError:
    print("⚠️  Warning: gRPC proto files not found. Generate with:")
    print("   python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. agent.proto")
    raise


class AxonHubAgent:
    """Agent that connects to AxonHub Core through the Ubuntu bridge."""
    
    def __init__(self, bridge_address: str = "localhost:50051", api_key: str = None):
        """
        Initialize AxonHub agent.
        
        Args:
            bridge_address: Address of the bridge (e.g., "localhost:50051")
            api_key: API key for LLM calls
        """
        self.bridge_address = bridge_address
        self.api_key = api_key
        self.agent_id = None
        self.channel = None
        self.stub = None
        
        self._connect()
    
    def _connect(self):
        """Connect to the bridge and register agent."""
        try:
            self.channel = grpc.insecure_channel(self.bridge_address)
            self.stub = agent_pb2_grpc.DesktopAgentStub(self.channel)
            
            # Register with the bridge
            response = self.stub.RegisterAgent(agent_pb2.ConnectRequest())
            self.agent_id = response.agent_id
            
            print(f"✅ Connected to bridge, agent ID: {self.agent_id}")
        except grpc.RpcError as e:
            print(f"❌ Failed to connect to bridge at {self.bridge_address}")
            raise
    
    def capture_screenshot(self) -> bytes:
        """Capture screenshot from the bridge."""
        request = agent_pb2.ScreenshotRequest(agent_id=self.agent_id)
        response = self.stub.CaptureScreenshot(request)
        return response.image_data
    
    def inject_key_press(self, key: str, modifiers: List[str] = None):
        """Inject a key press."""
        request = agent_pb2.KeyPressRequest(
            agent_id=self.agent_id,
            key=key,
            modifiers=modifiers or []
        )
        self.stub.InjectKeyPress(request)
    
    def inject_mouse_click(self, x: int, y: int, button: str = "left"):
        """Click at specific coordinates."""
        request = agent_pb2.MouseClickRequest(
            agent_id=self.agent_id,
            x=x,
            y=y,
            button=button
        )
        self.stub.InjectMouseClick(request)
    
    def inject_mouse_move(self, x: int, y: int):
        """Move mouse to coordinates."""
        request = agent_pb2.MouseMoveRequest(
            agent_id=self.agent_id,
            x=x,
            y=y
        )
        self.stub.InjectMouseMove(request)
    
    def type_text(self, text: str):
        """Type text."""
        # Send text character by character or as bulk text
        # Depends on your bridge implementation
        for char in text:
            self.inject_key_press(char)
            time.sleep(0.01)  # Small delay between characters
    
    def get_window_list(self) -> List[str]:
        """Get list of open windows."""
        request = agent_pb2.GetWindowListRequest(agent_id=self.agent_id)
        response = self.stub.GetWindowList(request)
        return list(response.windows)
    
    def get_active_window(self) -> str:
        """Get the currently active window."""
        request = agent_pb2.GetActiveWindowRequest(agent_id=self.agent_id)
        response = self.stub.GetActiveWindow(request)
        return response.window_title
    
    def run_task(self, task: str, max_steps: int = 15) -> Dict[str, Any]:
        """
        Run a task using the agent.
        
        This is a simplified version. In production, you would:
        1. Call your LLM (Claude, GPT, etc.) with the task and screenshot
        2. Parse the LLM's response for actions
        3. Execute actions via the bridge
        4. Repeat until task complete or max steps
        
        Args:
            task: Task description
            max_steps: Maximum steps to execute
        
        Returns:
            Dictionary with results
        """
        print(f"🎯 Task: {task}")
        print(f"📸 Max steps: {max_steps}")
        
        steps_taken = 0
        total_cost = 0.0
        
        for step in range(max_steps):
            steps_taken += 1
            
            # 1. Capture current state
            screenshot = self.capture_screenshot()
            print(f"  Step {step + 1}: Captured screenshot ({len(screenshot)} bytes)")
            
            # 2. Call LLM to decide action (placeholder)
            # In production, you would call your LLM here:
            # response = call_claude(task, screenshot, self.api_key)
            # action = parse_action(response)
            
            # For now, just a simple demonstration
            action = self._demo_action(step)
            
            # 3. Execute action
            print(f"  Action: {action['type']}")
            self._execute_action(action)
            
            # 4. Check if task complete (placeholder)
            # In production, you would have evaluator logic here
            time.sleep(0.5)
        
        return {
            "success": False,  # Placeholder - would be determined by evaluator
            "reward": 0.0,
            "steps": steps_taken,
            "cost": total_cost
        }
    
    def _demo_action(self, step: int) -> Dict[str, Any]:
        """Generate a demo action (placeholder)."""
        # This is just for demonstration
        actions = [
            {"type": "click", "x": 100, "y": 100},
            {"type": "type", "text": "hello"},
            {"type": "key", "key": "Return"},
        ]
        return actions[step % len(actions)]
    
    def _execute_action(self, action: Dict[str, Any]):
        """Execute an action on the bridge."""
        action_type = action.get("type")
        
        if action_type == "click":
            self.inject_mouse_click(action["x"], action["y"])
        elif action_type == "type":
            self.type_text(action["text"])
        elif action_type == "key":
            self.inject_key_press(action["key"], action.get("modifiers", []))
        elif action_type == "move":
            self.inject_mouse_move(action["x"], action["y"])
        else:
            print(f"⚠️  Unknown action type: {action_type}")
    
    def close(self):
        """Close connection to bridge."""
        if self.channel:
            self.channel.close()
    
    def __del__(self):
        """Cleanup on deletion."""
        self.close()


# Example usage
if __name__ == "__main__":
    agent = AxonHubAgent(bridge_address="localhost:50051")
    
    # Test basic functionality
    print("Testing bridge connection...")
    windows = agent.get_window_list()
    print(f"Open windows: {windows}")
    
    screenshot = agent.capture_screenshot()
    print(f"Screenshot size: {len(screenshot)} bytes")
    
    agent.close()
    print("✅ Test complete")
