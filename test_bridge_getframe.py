#!/usr/bin/env python3
"""
Bridge GetFrame Diagnostic Test
Tests if the bridge's GetFrame RPC is working or hanging
"""

import grpc
import sys
import time
from pathlib import Path

# Add proto path
sys.path.insert(0, str(Path(__file__).parent / "proto"))

try:
    import agent_pb2
    import agent_pb2_grpc
except ImportError:
    print("❌ ERROR: Could not import proto files. Run:")
    print("   cd /home/th3mailman/AXONBRIDGE-Linux")
    print("   python3 -m grpc_tools.protoc -I proto --python_out=proto --grpc_python_out=proto proto/agent.proto")
    sys.exit(1)

def test_bridge_getframe(bridge_host="localhost", bridge_port=50051, timeout=10):
    """Test GetFrame RPC with timeout"""
    
    print(f"\n{'='*60}")
    print(f"🔍 BRIDGE GetFrame DIAGNOSTIC TEST")
    print(f"{'='*60}\n")
    
    print(f"📡 Connecting to bridge at {bridge_host}:{bridge_port}...")
    
    # Create channel with timeout
    channel = grpc.insecure_channel(f'{bridge_host}:{bridge_port}')
    stub = agent_pb2_grpc.DesktopAgentStub(channel)
    
    try:
        # Test 1: Heartbeat (should be fast)
        print("\n1️⃣ Testing Heartbeat RPC...")
        start = time.time()
        try:
            response = stub.Heartbeat(
                agent_pb2.HeartbeatRequest(agent_id="test-diagnostic"),
                timeout=5
            )
            elapsed = time.time() - start
            print(f"   ✅ Heartbeat OK ({elapsed:.2f}s)")
            print(f"   Response: status={response.status}, timestamp={response.server_timestamp}")
        except grpc.RpcError as e:
            print(f"   ❌ Heartbeat FAILED: {e.code()} - {e.details()}")
            return False
        
        # Test 2: RegisterAgent
        print("\n2️⃣ Registering test agent...")
        start = time.time()
        try:
            response = stub.RegisterAgent(
                agent_pb2.ConnectRequest(
                    session_id="test-diagnostic-session",
                    hub_url="http://localhost:4545"
                ),
                timeout=5
            )
            elapsed = time.time() - start
            print(f"   ✅ RegisterAgent OK ({elapsed:.2f}s)")
            print(f"   Agent ID: {response.agent_id}")
            agent_id = response.agent_id
        except grpc.RpcError as e:
            print(f"   ❌ RegisterAgent FAILED: {e.code()} - {e.details()}")
            return False
        
        # Test 3: GetFrame (THE CRITICAL TEST)
        print(f"\n3️⃣ Testing GetFrame RPC (timeout={timeout}s)...")
        print(f"   ⏱️  Starting GetFrame call...")
        start = time.time()
        
        try:
            response = stub.GetFrame(
                agent_pb2.GetFrameRequest(agent_id=agent_id),
                timeout=timeout
            )
            elapsed = time.time() - start
            
            if response.frame:
                frame = response.frame
                print(f"   ✅ GetFrame SUCCESS! ({elapsed:.2f}s)")
                print(f"   Frame: {frame.width}x{frame.height}, {len(frame.data)} bytes")
                print(f"   Format: {frame.format}, Sequence: {frame.sequence_number}")
                print(f"   Timestamp: {frame.timestamp}")
                
                # Save frame to verify it's valid
                output_path = "/tmp/bridge_test_frame.png"
                with open(output_path, "wb") as f:
                    f.write(frame.data)
                print(f"   💾 Saved frame to: {output_path}")
                
                return True
            else:
                print(f"   ⚠️  GetFrame returned empty frame after {elapsed:.2f}s")
                return False
                
        except grpc.RpcError as e:
            elapsed = time.time() - start
            if e.code() == grpc.StatusCode.DEADLINE_EXCEEDED:
                print(f"   ❌ GetFrame TIMEOUT after {elapsed:.2f}s")
                print(f"   🚨 DIAGNOSIS: Bridge GetFrame RPC is HANGING!")
                return False
            else:
                print(f"   ❌ GetFrame FAILED: {e.code()} - {e.details()} (after {elapsed:.2f}s)")
                return False
        
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        return False
    finally:
        channel.close()

def main():
    print("\n" + "="*60)
    print("BRIDGE DIAGNOSTIC - Isolating GetFrame Issue")
    print("="*60)
    
    # Test with progressively longer timeouts
    timeouts = [5, 10, 30]
    
    for timeout in timeouts:
        print(f"\n📊 Attempt with {timeout}s timeout...")
        success = test_bridge_getframe(timeout=timeout)
        
        if success:
            print(f"\n{'='*60}")
            print("✅ DIAGNOSIS: Bridge GetFrame is WORKING")
            print("   → Issue is likely in the HUB/SERVER side")
            print("="*60 + "\n")
            return 0
        
        if timeout < 30:
            print(f"\n⏳ Retrying with longer timeout...")
            time.sleep(2)
    
    print(f"\n{'='*60}")
    print("❌ DIAGNOSIS: Bridge GetFrame is BROKEN/HANGING")
    print("   → Issue is in the BRIDGE implementation")
    print("   → Likely causes:")
    print("      • X11 capture hanging in LinuxCapturer")
    print("      • Native capture blocking indefinitely")
    print("      • Resource deadlock in bridge")
    print("="*60 + "\n")
    
    return 1

if __name__ == "__main__":
    sys.exit(main())
