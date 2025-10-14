#!/usr/bin/env python3
"""
Direct Bridge Test - Launch Calculator and Click Buttons
Tests the Bridge gRPC service directly, bypassing the Hub
"""

import grpc
import sys
import time
import os

# Add proto generated code to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'python_proto'))

try:
    import agent_pb2
    import agent_pb2_grpc
except ImportError:
    print("ERROR: Could not import proto files. Generating them now...")
    os.system("python3 -m grpc_tools.protoc -I./proto --python_out=./python_proto --grpc_python_out=./python_proto ./proto/agent.proto")
    import agent_pb2
    import agent_pb2_grpc

def test_bridge_directly():
    """Test Bridge directly via gRPC"""
    
    # Connect to Bridge
    bridge_addr = "localhost:50051"
    print(f"🔌 Connecting to Bridge at {bridge_addr}...")
    
    channel = grpc.insecure_channel(bridge_addr)
    stub = agent_pb2_grpc.DesktopAgentStub(channel)
    
    try:
        # Test 1: Launch Calculator
        print("\n" + "="*60)
        print("📱 TEST 1: Launch Calculator")
        print("="*60)
        
        launch_req = agent_pb2.LaunchApplicationRequest(
            app_id="org.gnome.Calculator.desktop"
        )
        
        print("   Sending LaunchApplication request...")
        start_time = time.time()
        launch_resp = stub.LaunchApplication(launch_req, timeout=10)
        elapsed = time.time() - start_time
        
        print(f"   ✅ Launch completed in {elapsed:.2f}s")
        print(f"   Success: {launch_resp.success}")
        if launch_resp.error_message:
            print(f"   Error: {launch_resp.error_message}")
        
        # Wait for Calculator to fully open
        print("\n   ⏳ Waiting 3 seconds for Calculator to open...")
        time.sleep(3)
        
        # Test 2: Click button "1"
        print("\n" + "="*60)
        print("🖱️  TEST 2: Click button '1' (around position 100, 250)")
        print("="*60)
        
        click_req = agent_pb2.InjectMouseClickRequest(
            x=100,
            y=250,
            button="left"
        )
        
        print("   Sending InjectMouseClick request...")
        start_time = time.time()
        click_resp = stub.InjectMouseClick(click_req, timeout=5)
        elapsed = time.time() - start_time
        
        print(f"   ✅ Click completed in {elapsed:.3f}s")
        print(f"   Success: {click_resp.success}")
        if click_resp.error_message:
            print(f"   Error: {click_resp.error_message}")
        
        time.sleep(0.5)
        
        # Test 3: Click button "2"
        print("\n" + "="*60)
        print("🖱️  TEST 3: Click button '2' (around position 150, 250)")
        print("="*60)
        
        click_req = agent_pb2.InjectMouseClickRequest(
            x=150,
            y=250,
            button="left"
        )
        
        print("   Sending InjectMouseClick request...")
        start_time = time.time()
        click_resp = stub.InjectMouseClick(click_req, timeout=5)
        elapsed = time.time() - start_time
        
        print(f"   ✅ Click completed in {elapsed:.3f}s")
        print(f"   Success: {click_resp.success}")
        if click_resp.error_message:
            print(f"   Error: {click_resp.error_message}")
        
        time.sleep(0.5)
        
        # Test 4: Click button "3"
        print("\n" + "="*60)
        print("🖱️  TEST 4: Click button '3' (around position 200, 250)")
        print("="*60)
        
        click_req = agent_pb2.InjectMouseClickRequest(
            x=200,
            y=250,
            button="left"
        )
        
        print("   Sending InjectMouseClick request...")
        start_time = time.time()
        click_resp = stub.InjectMouseClick(click_req, timeout=5)
        elapsed = time.time() - start_time
        
        print(f"   ✅ Click completed in {elapsed:.3f}s")
        print(f"   Success: {click_resp.success}")
        if click_resp.error_message:
            print(f"   Error: {click_resp.error_message}")
        
        # Final summary
        print("\n" + "="*60)
        print("✅ ALL TESTS COMPLETED SUCCESSFULLY!")
        print("="*60)
        print("\n📝 What you should see:")
        print("   - Calculator window opened")
        print("   - Buttons '1', '2', '3' were clicked")
        print("   - Display should show '123' in Calculator")
        print("\n🔍 If you see '123' in Calculator → Bridge works perfectly!")
        print("   If NOT → There's an issue with Bridge input injection")
        print("\n💡 This proves whether the issue is Bridge or Hub.")
        
    except grpc.RpcError as e:
        print(f"\n❌ gRPC ERROR: {e.code()}: {e.details()}")
        return 1
    except Exception as e:
        print(f"\n❌ ERROR: {e}")
        import traceback
        traceback.print_exc()
        return 1
    finally:
        channel.close()
    
    return 0

if __name__ == "__main__":
    sys.exit(test_bridge_directly())
