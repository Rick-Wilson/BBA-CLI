#!/usr/bin/env python3
"""
A/B test: Compare auctions from Windows bba-cli vs macOS bba-cli-mac.

Runs the same PBN input through both CLIs and compares the PBN auction outputs.
Also supports JSON batch mode for exact comparison.

Usage:
    python3 ab_test.py                           # Use default test file
    python3 ab_test.py --input path/to/file.pbn  # Custom input
    python3 ab_test.py --count 50                # Limit number of deals
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# Paths
SCRIPT_DIR = Path(__file__).parent
BBA_CLI_DIR = SCRIPT_DIR.parent
BBSA_DIR = Path.home() / "Development/GitHub/Practice-Bidding-Scenarios/bbsa"
NS_CONV = BBSA_DIR / "21GF-DEFAULT.bbsa"
EW_CONV = BBSA_DIR / "21GF-GIB.bbsa"
DYLIB_DIR = Path.home() / "Development/GitHub/bba-mac-private/epbot-aot/bin/Release/net10.0/osx-arm64/publish"
MAC_CLI = SCRIPT_DIR / "target/release/bba-cli-mac"
WIN_CLI_DIR = r"\\Mac\Home\Development\GitHub\BBA-CLI\dist\epbot-wrapper"

# SSH setup
os.environ.setdefault('WINDOWS_HOST', '10.211.55.5')
os.environ.setdefault('WINDOWS_USER', 'Rick')
sys.path.insert(0, str(Path.home() / "Development/GitHub/Practice-Bidding-Scenarios/build-scripts-mac"))


def extract_auctions_from_pbn(pbn_path):
    """Extract auctions from a PBN file. Handles the output format where
    tags and content may be intermixed.
    Returns list of (board, dealer, vul, deal_pbn, bids)."""
    games = []
    current = {}
    lines_after_tags = []
    all_bids = []

    def extract_bids(content_lines):
        """Extract bids from content lines, taking only the LAST block
        (after the last * separator, which separates original from generated)."""
        last_star = -1
        for i, cl in enumerate(content_lines):
            if cl.strip() == '*':
                last_star = i
        start = last_star + 1 if last_star >= 0 else 0
        bids = []
        for cl in content_lines[start:]:
            cl = cl.strip()
            if not cl or cl.startswith('%') or cl.startswith(';') or cl == '*':
                continue
            tokens = cl.split()
            for tok in tokens:
                if tok.startswith('=') or tok.startswith('$'):
                    continue
                bids.append(tok)
        return bids

    with open(pbn_path) as f:
        for line in f:
            line = line.rstrip()
            stripped = line.strip()

            if not stripped:
                if current.get("deal"):
                    bids = extract_bids(lines_after_tags)
                    if bids:
                        games.append((
                            current.get("board", "?"),
                            current.get("dealer", "?"),
                            current.get("vul", "None"),
                            current["deal"],
                            bids
                        ))
                current = {}
                lines_after_tags = []
                continue

            m = re.match(r'\[(\w+)\s+"([^"]*)"\]', stripped)
            if m:
                tag, val = m.group(1), m.group(2)
                if tag == "Board":
                    current["board"] = val
                elif tag == "Dealer":
                    current["dealer"] = val
                elif tag == "Vulnerable":
                    current["vul"] = val
                elif tag == "Deal":
                    current["deal"] = val
            else:
                lines_after_tags.append(line)

    # Handle last game
    if current.get("deal"):
        bids = extract_bids(lines_after_tags)
        if bids:
            games.append((
                current.get("board", "?"),
                current.get("dealer", "?"),
                current.get("vul", "None"),
                current["deal"],
                bids
            ))

    return games


def normalize_bid(bid):
    """Normalize a bid string for comparison."""
    bid = bid.upper().strip()
    if len(bid) == 2 and bid[0].isdigit() and bid[1] == "N":
        bid = bid + "T"
    return bid


def compare_auctions(a1, a2):
    """Compare two bid lists. Returns (match, first_diff_position)."""
    n1 = [normalize_bid(b) for b in a1]
    n2 = [normalize_bid(b) for b in a2]
    if n1 == n2:
        return True, -1
    for i in range(min(len(n1), len(n2))):
        if n1[i] != n2[i]:
            return False, i
    return False, min(len(n1), len(n2))


def run_mac(input_pbn, output_pbn):
    """Run bba-cli-mac on macOS."""
    if not MAC_CLI.exists():
        print(f"ERROR: bba-cli-mac not found at {MAC_CLI}")
        print("  Build with: cd cli-mac && cargo build --release")
        sys.exit(1)

    env = os.environ.copy()
    env["DYLD_LIBRARY_PATH"] = str(DYLIB_DIR)

    cmd = [
        str(MAC_CLI),
        "--input", str(input_pbn),
        "--output", str(output_pbn),
        "--ns-conventions", str(NS_CONV),
        "--ew-conventions", str(EW_CONV),
    ]

    result = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=600)

    if result.returncode != 0:
        print(f"  macOS CLI error (rc={result.returncode}):")
        print(f"  stderr: {result.stderr[:500]}")
        return False

    # Print key info from stderr (where log output goes)
    for line in result.stderr.split('\n'):
        if 'EPBot version' in line or 'Processed' in line or 'deals' in line:
            print(f"  {line.strip()}")

    return True


def run_windows(input_pbn, output_pbn):
    """Run bba-cli.exe on Windows via SSH."""
    from ssh_runner import run_windows_command

    unc_base = r"\\Mac\Home\Development\GitHub"
    home = str(Path.home() / "Development/GitHub")

    def to_unc(p):
        return str(p).replace(home, unc_base).replace("/", "\\")

    unc_input = to_unc(input_pbn)
    unc_output = to_unc(output_pbn)
    unc_ns = to_unc(NS_CONV)
    unc_ew = to_unc(EW_CONV)

    cmd = (
        f'pushd {WIN_CLI_DIR} && '
        f'bba-cli.exe '
        f'--input "{unc_input}" '
        f'--output "{unc_output}" '
        f'--ns-conventions "{unc_ns}" '
        f'--ew-conventions "{unc_ew}" '
        f'&& popd'
    )

    returncode, stdout, stderr = run_windows_command(cmd, check=False, timeout=600)

    if returncode != 0:
        print(f"  Windows CLI error (rc={returncode}):")
        print(f"  stderr: {stderr[:500]}")
        if stdout:
            print(f"  stdout: {stdout[:500]}")
        return False

    # Print key lines
    for line in (stderr + stdout).split('\n'):
        if 'Processed' in line or 'deals' in line or 'EPBot' in line:
            print(f"  {line.strip()}")

    return True


def main():
    parser = argparse.ArgumentParser(description="A/B test: Windows bba-cli vs macOS bba-cli-mac")
    parser.add_argument("--input", type=Path,
                       default=BBA_CLI_DIR / "test-conv-fix.pbn",
                       help="Input PBN file")
    parser.add_argument("--count", type=int, default=0,
                       help="Limit number of deals to compare (0 = all)")
    parser.add_argument("--mac-only", action="store_true",
                       help="Only run macOS, compare with existing Windows output")
    parser.add_argument("--win-output", type=Path,
                       help="Path to existing Windows output (use with --mac-only)")
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Input file not found: {args.input}")
        sys.exit(1)

    print(f"A/B Test: Windows bba-cli vs macOS bba-cli-mac")
    print(f"  Input: {args.input}")
    print(f"  NS conventions: {NS_CONV}")
    print(f"  EW conventions: {EW_CONV}")

    # Create output files in the BBA-CLI directory (accessible to both platforms)
    output_dir = BBA_CLI_DIR / "ab-test-output"
    output_dir.mkdir(exist_ok=True)
    mac_output = output_dir / "mac-output.pbn"
    win_output = args.win_output or output_dir / "win-output.pbn"

    # --- macOS ---
    print(f"\n1. Running macOS bba-cli-mac...")
    if not run_mac(args.input, mac_output):
        print("FATAL: macOS CLI failed")
        sys.exit(1)
    print(f"  Output: {mac_output}")

    # --- Windows ---
    if not args.mac_only:
        print(f"\n2. Running Windows bba-cli (via SSH)...")
        if not run_windows(args.input, win_output):
            print("FATAL: Windows CLI failed")
            sys.exit(1)
        print(f"  Output: {win_output}")
    else:
        if not win_output.exists():
            print(f"Windows output not found: {win_output}")
            sys.exit(1)
        print(f"\n2. Using existing Windows output: {win_output}")

    # --- Compare ---
    print(f"\n3. Comparing auctions...")
    mac_auctions = extract_auctions_from_pbn(mac_output)
    win_auctions = extract_auctions_from_pbn(win_output)

    print(f"  macOS: {len(mac_auctions)} auctions extracted")
    print(f"  Windows: {len(win_auctions)} auctions extracted")

    # Match by deal PBN string
    mac_by_deal = {}
    for a in mac_auctions:
        mac_by_deal[a[3]] = a
    win_by_deal = {}
    for a in win_auctions:
        win_by_deal[a[3]] = a

    common_deals = sorted(set(mac_by_deal.keys()) & set(win_by_deal.keys()))
    if args.count > 0:
        common_deals = common_deals[:args.count]

    match_count = 0
    diff_count = 0

    print(f"\n{'='*70}")
    print(f"{'Board':>6}  {'Dlr':>3}  {'Vul':>4}  {'Match':>5}  Bids")
    print(f"{'-'*70}")

    for deal in common_deals:
        m = mac_by_deal[deal]
        w = win_by_deal[deal]
        mac_bids = m[4]
        win_bids = w[4]

        match, diff_pos = compare_auctions(mac_bids, win_bids)

        board = m[0]
        dealer = m[1]
        vul = m[2]

        if match:
            match_count += 1
            if match_count <= 5:
                print(f"  {board:>5}  {dealer:>3}  {vul:>4}  {'OK':>5}  {' '.join(mac_bids)}")
        else:
            diff_count += 1
            print(f"  {board:>5}  {dealer:>3}  {vul:>4}  {'DIFF':>5}  Mac: {' '.join(mac_bids)}")
            print(f"  {'':>5}  {'':>3}  {'':>4}  {'':>5}  Win: {' '.join(win_bids)}")
            if 0 <= diff_pos < len(mac_bids) and diff_pos < len(win_bids):
                print(f"  {'':>5}  {'':>3}  {'':>4}  {'':>5}  ^ pos {diff_pos}: {mac_bids[diff_pos]} vs {win_bids[diff_pos]}")
            print()

    total = match_count + diff_count
    print(f"\n{'='*70}")
    print(f"RESULTS: {match_count} match, {diff_count} diff out of {total} common deals")

    if total > 0:
        pct = 100 * match_count / total
        print(f"Match rate: {pct:.1f}%")
        if pct == 100.0:
            print("\nPERFECT MATCH! macOS bba-cli-mac produces identical results to Windows bba-cli.")
        elif pct >= 99.0:
            print(f"\nNear-perfect match. {diff_count} deal(s) differ.")
        else:
            print(f"\nSignificant differences. {diff_count} deals differ.")

    mac_only = len(mac_by_deal) - len(common_deals)
    win_only = len(win_by_deal) - len(common_deals)
    if mac_only > 0:
        print(f"\n  {mac_only} deals in macOS output but not matched")
    if win_only > 0:
        print(f"  {win_only} deals in Windows output but not matched")


if __name__ == "__main__":
    main()
