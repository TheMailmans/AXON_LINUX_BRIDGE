#!/usr/bin/env python3
"""
AxonHub Official OSWorld Test Runner
Runs OSWorld benchmarks through the Ubuntu bridge with automatic result tracking.
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any

import grpc

# Add mm_agents to path if needed
sys.path.insert(0, str(Path(__file__).parent / "mm_agents"))

from axonhub_text_first_agent import AxonHubAgent


def load_osworld_tests(test_file: str = "config/default_test.json") -> List[Dict[str, Any]]:
    """Load OSWorld test configurations."""
    try:
        with open(test_file, 'r') as f:
            tests = json.load(f)
        return tests
    except FileNotFoundError:
        print(f"❌ Test file not found: {test_file}")
        print("   Looking in OSWorld installation for test definitions...")
        
        # Try to find OSWorld installation
        osworld_paths = [
            "/home/tylermailman/Documents/Projects/OSWorld",
            str(Path.home() / "Documents" / "Projects" / "OSWorld"),
            "./OSWorld"
        ]
        
        for path in osworld_paths:
            test_path = Path(path) / "config" / "default_test.json"
            if test_path.exists():
                with open(test_path, 'r') as f:
                    return json.load(f)
        
        raise FileNotFoundError("Could not find OSWorld test configurations")


def run_single_test(agent: AxonHubAgent, test_id: str, test_config: Dict[str, Any]) -> Dict[str, Any]:
    """Run a single OSWorld test and return results."""
    print(f"\n{'='*60}")
    print(f"🧪 Running Test: {test_id}")
    print(f"   Task: {test_config.get('instruction', 'N/A')[:80]}...")
    print(f"{'='*60}\n")
    
    start_time = time.time()
    result = {
        "test_id": test_id,
        "status": "unknown",
        "reward": 0.0,
        "steps_taken": 0,
        "cost": 0.0,
        "duration": 0.0,
        "error": None,
        "timestamp": datetime.now().isoformat()
    }
    
    try:
        # Run the agent on this test
        agent_result = agent.run_task(
            task=test_config.get('instruction', ''),
            max_steps=test_config.get('max_steps', 15)
        )
        
        # Parse results
        result["status"] = "passed" if agent_result.get("success", False) else "failed"
        result["reward"] = agent_result.get("reward", 0.0)
        result["steps_taken"] = agent_result.get("steps", 0)
        result["cost"] = agent_result.get("cost", 0.0)
        
        status_icon = "✅" if result["status"] == "passed" else "❌"
        print(f"\n{status_icon} Test {test_id}: {result['status'].upper()}")
        print(f"   Reward: {result['reward']:.2f}")
        print(f"   Steps: {result['steps_taken']}")
        print(f"   Cost: ${result['cost']:.3f}")
        
    except Exception as e:
        result["status"] = "error"
        result["error"] = str(e)
        print(f"\n⚠️  Test {test_id}: ERROR")
        print(f"   {str(e)}")
    
    result["duration"] = time.time() - start_time
    print(f"   Duration: {result['duration']:.1f}s\n")
    
    return result


def save_results(results: List[Dict[str, Any]], output_dir: Path):
    """Save results to JSON and generate submission report."""
    output_dir.mkdir(parents=True, exist_ok=True)
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    # Calculate summary statistics
    total = len(results)
    passed = sum(1 for r in results if r["status"] == "passed")
    failed = sum(1 for r in results if r["status"] == "failed")
    errors = sum(1 for r in results if r["status"] == "error")
    total_cost = sum(r["cost"] for r in results)
    total_duration = sum(r["duration"] for r in results)
    
    summary = {
        "tests": results,
        "summary": {
            "total": total,
            "passed": passed,
            "failed": failed,
            "errors": errors,
            "pass_rate": passed / total if total > 0 else 0.0,
            "total_cost": total_cost,
            "total_duration": total_duration,
            "timestamp": datetime.now().isoformat()
        }
    }
    
    # Save JSON results
    json_file = output_dir / f"results_{timestamp}.json"
    with open(json_file, 'w') as f:
        json.dump(summary, f, indent=2)
    print(f"\n💾 Results saved: {json_file}")
    
    # Generate submission report
    report_file = output_dir / f"SUBMISSION_REPORT_{timestamp}.md"
    with open(report_file, 'w') as f:
        f.write(f"# AxonHub OSWorld Benchmark Results\n\n")
        f.write(f"**Date:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        f.write(f"## Summary\n\n")
        f.write(f"- **Total Tests:** {total}\n")
        f.write(f"- **Passed:** {passed} ✅\n")
        f.write(f"- **Failed:** {failed} ❌\n")
        f.write(f"- **Errors:** {errors} ⚠️\n")
        f.write(f"- **Pass Rate:** {passed/total*100:.1f}%\n")
        f.write(f"- **Total Cost:** ${total_cost:.2f}\n")
        f.write(f"- **Total Duration:** {total_duration/60:.1f} minutes\n\n")
        f.write(f"## Individual Test Results\n\n")
        f.write(f"| Test ID | Status | Reward | Steps | Cost | Duration |\n")
        f.write(f"|---------|--------|--------|-------|------|----------|\n")
        
        for r in results:
            status_icon = "✅" if r["status"] == "passed" else ("❌" if r["status"] == "failed" else "⚠️")
            f.write(f"| {r['test_id']} | {status_icon} {r['status']} | {r['reward']:.2f} | {r['steps_taken']} | ${r['cost']:.3f} | {r['duration']:.1f}s |\n")
        
        f.write(f"\n## Submission Information\n\n")
        f.write(f"This report was generated automatically by the AxonHub OSWorld test runner.\n")
        f.write(f"For leaderboard submission, send this report along with the JSON file to:\n")
        f.write(f"- tianbaoxiexxx@gmail.com\n")
        f.write(f"- yuanmengqi732@gmail.com\n\n")
        f.write(f"**Reference:** https://github.com/xlang-ai/OSWorld/blob/main/PUBLIC_EVALUATION_GUIDELINE.md\n")
    
    print(f"📄 Report saved: {report_file}")


def main():
    parser = argparse.ArgumentParser(description="Run OSWorld tests through AxonHub bridge")
    parser.add_argument("--test", help="Run single test by ID (e.g., osworld_012)")
    parser.add_argument("--difficulty", choices=["easy", "medium", "hard"], help="Run tests of specific difficulty")
    parser.add_argument("--full", action="store_true", help="Run full benchmark (~370 tests)")
    parser.add_argument("--bridge", default="localhost:50051", help="Bridge address (default: localhost:50051)")
    parser.add_argument("--api-key", help="API key for LLM (or set ANTHROPIC_API_KEY env var)")
    parser.add_argument("--output-dir", default="./axonhub_official_results", help="Output directory for results")
    
    args = parser.parse_args()
    
    # Get API key
    api_key = args.api_key or os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        print("❌ Error: API key required. Use --api-key or set ANTHROPIC_API_KEY")
        sys.exit(1)
    
    # Load tests
    print("📚 Loading OSWorld test configurations...")
    all_tests = load_osworld_tests()
    
    # Filter tests based on arguments
    if args.test:
        tests_to_run = {args.test: t for t in all_tests if t.get("test_id") == args.test}
        if not tests_to_run:
            print(f"❌ Test '{args.test}' not found")
            sys.exit(1)
    elif args.difficulty:
        tests_to_run = {t["test_id"]: t for t in all_tests if t.get("difficulty") == args.difficulty}
    elif args.full:
        tests_to_run = {t["test_id"]: t for t in all_tests}
    else:
        print("❌ Error: Specify --test, --difficulty, or --full")
        parser.print_help()
        sys.exit(1)
    
    print(f"🎯 Found {len(tests_to_run)} test(s) to run\n")
    
    # Initialize agent
    print(f"🔌 Connecting to bridge at {args.bridge}...")
    try:
        agent = AxonHubAgent(
            bridge_address=args.bridge,
            api_key=api_key
        )
        print("✅ Connected to bridge\n")
    except grpc.RpcError as e:
        print(f"❌ Failed to connect to bridge: {e}")
        print(f"   Make sure the bridge is running at {args.bridge}")
        sys.exit(1)
    
    # Run tests
    results = []
    for i, (test_id, test_config) in enumerate(tests_to_run.items(), 1):
        print(f"Progress: {i}/{len(tests_to_run)}")
        result = run_single_test(agent, test_id, test_config)
        results.append(result)
        
        # Save intermediate results after each test
        save_results(results, Path(args.output_dir))
    
    # Print final summary
    print(f"\n{'='*60}")
    print("🎉 Testing Complete!")
    print(f"{'='*60}")
    passed = sum(1 for r in results if r["status"] == "passed")
    print(f"✅ Passed: {passed}/{len(results)} ({passed/len(results)*100:.1f}%)")
    print(f"💰 Total Cost: ${sum(r['cost'] for r in results):.2f}")
    print(f"⏱️  Total Time: {sum(r['duration'] for r in results)/60:.1f} minutes")


if __name__ == "__main__":
    main()
