#!/usr/bin/env python3
"""
Compare auction results between two PBN files or between a PBN file and JSON wrapper output.
"""
import json
import re
import sys


def parse_pbn_file(filename):
    """Parse PBN file, return list of dicts with board, deal, auction."""
    with open(filename, 'r') as f:
        content = f.read()

    results = []
    current = {'board': None, 'deal': None, 'auction': []}
    in_auction = False

    for line in content.split('\n'):
        line_stripped = line.strip()

        if line_stripped.startswith('[Board '):
            # Save previous if complete
            if current['board'] and current['deal']:
                results.append(current.copy())
            current = {'board': None, 'deal': None, 'auction': []}
            in_auction = False
            match = re.search(r'"([^"]+)"', line_stripped)
            if match:
                current['board'] = match.group(1)

        elif line_stripped.startswith('[Deal '):
            match = re.search(r'"([^"]+)"', line_stripped)
            if match:
                current['deal'] = match.group(1)

        elif line_stripped.startswith('[Auction'):
            in_auction = True

        elif in_auction:
            if line_stripped.startswith('[') or line_stripped == '':
                in_auction = False
            else:
                # Parse bids from this line
                cleaned = re.sub(r'=\d+=', '', line_stripped)
                cleaned = re.sub(r'\{[^}]*\}', '', cleaned)
                parts = cleaned.split()
                for bid in parts:
                    bid = bid.strip()
                    if bid and bid != '*':
                        current['auction'].append(bid.upper())

    # Don't forget last one
    if current['board'] and current['deal']:
        results.append(current)

    return results

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

def compare_pbn_files(file1, file2):
    """Compare auctions between two PBN files."""
    results1 = parse_pbn_file(file1)
    results2 = parse_pbn_file(file2)

    print(f"File 1: {len(results1)} deals from {file1}")
    print(f"File 2: {len(results2)} deals from {file2}")
    print()

    # Index by deal string
    deals1 = {r['deal']: r for r in results1}
    deals2 = {r['deal']: r for r in results2}

    common_deals = set(deals1.keys()) & set(deals2.keys())
    only_in_1 = set(deals1.keys()) - set(deals2.keys())
    only_in_2 = set(deals2.keys()) - set(deals1.keys())

    print(f"Common deals: {len(common_deals)}")
    print(f"Only in file 1: {len(only_in_1)}")
    print(f"Only in file 2: {len(only_in_2)}")
    print()

    stats = {'full': 0, 'diff': 0, 'len_diff': 0}
    diff_positions = {}
    differences = []

    # Iterate in file1 order for consistent output
    for r1 in results1:
        deal = r1['deal']
        if deal not in deals2:
            continue

        r2 = deals2[deal]
        match_type, diff_idx = compare_auctions(r1['auction'], r2['auction'])
        stats[match_type] += 1

        if match_type == 'diff':
            diff_positions[diff_idx] = diff_positions.get(diff_idx, 0) + 1
            differences.append((r1['board'], r1['auction'], r2['auction'], diff_idx))
        elif match_type == 'len_diff':
            differences.append((r1['board'], r1['auction'], r2['auction'], diff_idx))

    total = len(common_deals)

    print("=== RESULTS ===")
    print(f"Full match:      {stats['full']:4d} ({100*stats['full']/total:.1f}%)")
    print(f"Bid differs:     {stats['diff']:4d} ({100*stats['diff']/total:.1f}%)")
    print(f"Length differs:  {stats['len_diff']:4d} ({100*stats['len_diff']/total:.1f}%)")
    print()

    if diff_positions:
        print("=== WHERE DIFFERENCES OCCUR ===")
        print("(0 = opening bid, 1 = response to opening, etc.)")
        for pos in sorted(diff_positions.keys()):
            print(f"  Position {pos}: {diff_positions[pos]} differences")

    print("\n=== SAMPLE DIFFERENCES ===")
    for board, auction1, auction2, diff_idx in differences[:10]:
        print(f"\nBoard {board}:")
        print(f"  File1: {' '.join(auction1)}")
        print(f"  File2: {' '.join(auction2)}")
        if diff_idx >= 0 and diff_idx < len(auction1) and diff_idx < len(auction2):
            print(f"  Diff at position {diff_idx}: {auction1[diff_idx]} vs {auction2[diff_idx]}")


def compare_pbn_to_json(ref_file, json_file):
    """Compare auctions between a PBN file and JSON wrapper output."""
    ref_results = parse_pbn_file(ref_file)
    our_deals = parse_wrapper_output(json_file)

    print(f"Reference: {len(ref_results)} deals from {ref_file}")
    print(f"Wrapper:   {len(our_deals)} deals from {json_file}")
    print()

    # Create lookup by deal string
    ref_by_deal = {r['deal']: r for r in ref_results}

    stats = {'full': 0, 'diff': 0, 'len_diff': 0, 'missing': 0}
    diff_positions = {}

    for deal, our_auction in our_deals:
        if deal not in ref_by_deal:
            stats['missing'] += 1
            continue

        ref = ref_by_deal[deal]
        match_type, diff_idx = compare_auctions(ref['auction'], our_auction)
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

        ref = ref_by_deal[deal]
        match_type, diff_idx = compare_auctions(ref['auction'], our_auction)

        if match_type == 'diff' and count < 10:
            count += 1
            print(f"\nBoard {ref['board']}:")
            print(f"  Ref: {' '.join(ref['auction'])}")
            print(f"  Our: {' '.join(our_auction)}")
            print(f"  Diff at position {diff_idx}: ref={ref['auction'][diff_idx]} vs our={our_auction[diff_idx]}")


def main():
    if len(sys.argv) < 3:
        print("Usage: compare_auctions.py <file1.pbn> <file2.pbn>")
        print("       compare_auctions.py <reference.pbn> <wrapper-output.json>")
        sys.exit(1)

    file1 = sys.argv[1]
    file2 = sys.argv[2]

    # Detect mode based on file extension
    if file2.endswith('.json'):
        compare_pbn_to_json(file1, file2)
    else:
        compare_pbn_files(file1, file2)

if __name__ == '__main__':
    main()
