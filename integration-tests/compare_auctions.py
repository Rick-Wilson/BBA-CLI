#!/usr/bin/env python3
"""
Compare auction results from our wrapper against reference PBN file.
"""
import json
import re
import sys

def parse_pbn_auction(pbn_lines):
    """Extract auction from PBN format."""
    # Find the [Auction "X"] line and subsequent bid lines
    in_auction = False
    bids = []

    for line in pbn_lines:
        line = line.strip()
        if line.startswith('[Auction'):
            in_auction = True
            continue
        if in_auction:
            if line.startswith('[') or line == '':
                break
            # Parse bid line - remove annotations like =1= or {comment}
            line = re.sub(r'=\d+=', '', line)  # Remove =1=, =2=, etc.
            line = re.sub(r'\{[^}]*\}', '', line)  # Remove {comments}
            parts = line.split()
            for bid in parts:
                bid = bid.strip()
                if bid and bid != '*':
                    # Normalize bid format
                    bid = bid.upper().replace('NT', 'NT')
                    bids.append(bid)

    return bids

def parse_reference_pbn(filename):
    """Parse reference PBN file and return list of (deal, auction) tuples."""
    with open(filename, 'r') as f:
        content = f.read()

    # Split into deals
    deals = []
    current_deal = None
    current_lines = []

    for line in content.split('\n'):
        if line.startswith('[Deal '):
            if current_deal:
                auction = parse_pbn_auction(current_lines)
                deals.append((current_deal, auction))
            match = re.search(r'"([^"]+)"', line)
            if match:
                current_deal = match.group(1)
            current_lines = [line]
        elif current_deal:
            current_lines.append(line)

    # Don't forget the last deal
    if current_deal:
        auction = parse_pbn_auction(current_lines)
        deals.append((current_deal, auction))

    return deals

def parse_wrapper_output(filename):
    """Parse our wrapper's JSON output."""
    with open(filename, 'r') as f:
        data = json.load(f)

    results = []
    for r in data['results']:
        deal = r['deal']
        # Normalize the auction
        auction = [b.upper().replace('NT', 'NT') for b in r['auction']]
        results.append((deal, auction))

    return results

def normalize_bid(bid):
    """Normalize bid format for comparison."""
    # 1N -> 1NT, 2N -> 2NT, etc.
    bid = bid.upper()
    if bid in ('1N', '2N', '3N', '4N', '5N', '6N', '7N'):
        bid = bid + 'T'
    return bid

def compare_auctions(ref_auction, our_auction):
    """
    Compare two auctions, return (match_type, first_diff_index).
    match_type: 'full', 'partial', 'none'
    """
    # Normalize both auctions
    ref_norm = [normalize_bid(b) for b in ref_auction]
    our_norm = [normalize_bid(b) for b in our_auction]

    if ref_norm == our_norm:
        return 'full', -1

    # Find first difference
    min_len = min(len(ref_norm), len(our_norm))
    for i in range(min_len):
        if ref_norm[i] != our_norm[i]:
            return 'diff', i

    # One is a prefix of the other
    return 'len_diff', min_len

def main():
    if len(sys.argv) < 3:
        print("Usage: compare_auctions.py <reference.pbn> <wrapper-output.json>")
        sys.exit(1)

    ref_file = sys.argv[1]
    our_file = sys.argv[2]

    ref_deals = parse_reference_pbn(ref_file)
    our_deals = parse_wrapper_output(our_file)

    print(f"Reference: {len(ref_deals)} deals from {ref_file}")
    print(f"Wrapper:   {len(our_deals)} deals from {our_file}")
    print()

    # Create lookup by deal string
    ref_by_deal = {d[0]: d[1] for d in ref_deals}

    stats = {'full': 0, 'diff': 0, 'len_diff': 0, 'missing': 0}
    diff_positions = {}  # position -> count

    for deal, our_auction in our_deals:
        if deal not in ref_by_deal:
            stats['missing'] += 1
            continue

        ref_auction = ref_by_deal[deal]
        match_type, diff_idx = compare_auctions(ref_auction, our_auction)
        stats[match_type] += 1

        if match_type == 'diff':
            diff_positions[diff_idx] = diff_positions.get(diff_idx, 0) + 1

    total = len(our_deals) - stats['missing']

    print("=== RESULTS ===")
    print(f"Full match:      {stats['full']:4d} ({100*stats['full']/total:.1f}%)")
    print(f"Bid differs:     {stats['diff']:4d} ({100*stats['diff']/total:.1f}%)")
    print(f"Length differs:  {stats['len_diff']:4d} ({100*stats['len_diff']/total:.1f}%)")
    print(f"Not in ref:      {stats['missing']:4d}")
    print()

    if diff_positions:
        print("=== WHERE DIFFERENCES OCCUR ===")
        print("(0 = opening bid, 1 = response to opening, etc.)")
        for pos in sorted(diff_positions.keys()):
            print(f"  Position {pos}: {diff_positions[pos]} differences")

    # Show some specific differences
    print("\n=== SAMPLE DIFFERENCES ===")
    count = 0
    for deal, our_auction in our_deals:
        if deal not in ref_by_deal:
            continue

        ref_auction = ref_by_deal[deal]
        match_type, diff_idx = compare_auctions(ref_auction, our_auction)

        if match_type == 'diff' and count < 10:
            count += 1
            print(f"\nDeal: {deal}")
            print(f"  Ref: {' '.join(ref_auction)}")
            print(f"  Our: {' '.join(our_auction)}")
            print(f"  Diff at position {diff_idx}: ref={ref_auction[diff_idx]} vs our={our_auction[diff_idx]}")

if __name__ == '__main__':
    main()
