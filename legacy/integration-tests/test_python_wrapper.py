#!/usr/bin/env python3
"""
Test script using Python/pythonnet to call EPBot DLL directly.
Based on lorserker/ben approach.
"""

import sys
import json
import clr

# Add reference to the EPBot DLL
clr.AddReference(r"G:\BBA-CLI\EPBot64.dll")
from EPBot64 import EPBot

def parse_pbn_hand(pbn):
    """Parse PBN hand format: N:spades.hearts.diamonds.clubs E:... S:... W:..."""
    hands = {}
    parts = pbn.split()
    for part in parts:
        if ':' in part:
            pos, cards = part.split(':', 1)
            hands[pos] = cards.split('.')
        else:
            # Continuation of previous hand
            last_pos = list(hands.keys())[-1]
            suits = part.split('.')
            # This is actually the next position's hand
            positions = ['N', 'E', 'S', 'W']
            idx = positions.index(last_pos) + 1
            if idx < 4:
                hands[positions[idx]] = suits
    return hands

def load_conventions(bot, filepath, side):
    """Load conventions from .bbsa file"""
    with open(filepath, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('#') or line.startswith(';'):
                continue
            if ' = ' not in line:
                continue
            key, value = line.split(' = ', 1)
            key = key.strip()
            value = value.strip()

            try:
                int_val = int(value)
                if key.lower() == 'system type':
                    bot.set_system_type(side, int_val)
                elif key.lower() == 'opponent type':
                    bot.set_opponent_type(side, int_val)
                else:
                    bot.set_conventions(side, key, int_val == 1)
            except:
                pass

def run_auction(pbn, dealer_str, vul_str, ns_conv, ew_conv):
    """Run a complete auction and return the bid sequence"""

    # Parse dealer
    dealer_map = {'N': 0, 'E': 1, 'S': 2, 'W': 3}
    dealer = dealer_map.get(dealer_str, 0)

    # Parse vulnerability
    vul_map = {'None': 0, 'NS': 2, 'EW': 1, 'All': 3, 'Both': 3}
    vul = vul_map.get(vul_str, 0)

    # Parse hands
    hands = parse_pbn_hand(pbn)

    # Create 4 EPBot players
    players = []
    pos_order = ['N', 'E', 'S', 'W']

    for i, pos in enumerate(pos_order):
        bot = EPBot()

        # Get hand for this position
        hand = hands.get(pos, ['', '', '', ''])

        # PBN format is S.H.D.C but EPBot expects C.D.H.S - reverse the array
        hand = list(reversed(hand))

        # Call new_hand first (matching Edward's VB order)
        bot.new_hand(i, hand, dealer, vul)

        # Set scoring (0 = MP, 1 = IMP)
        bot.scoring = 1

        # Load conventions
        load_conventions(bot, ns_conv, 0)  # NS side
        load_conventions(bot, ew_conv, 1)  # EW side

        players.append(bot)

    # Run auction
    bids = []
    position = dealer
    passes = 0
    current_bid = 0

    for _ in range(64):  # Max 64 bids
        bid = players[position].get_bid()

        # Send bid to all players
        for p in players:
            p.set_bid(position, bid)

        # Convert bid code to string
        if bid == 0:
            bid_str = "Pass"
            passes += 1
        elif bid == 1:
            bid_str = "X"
            passes = 0
        elif bid == 2:
            bid_str = "XX"
            passes = 0
        else:
            level = bid // 5
            strain = bid % 5
            strains = ['C', 'D', 'H', 'S', 'NT']
            bid_str = f"{level}{strains[strain]}"
            passes = 0
            current_bid = bid

        bids.append(bid_str)

        # Check if auction is over
        if passes >= 4 or (passes >= 3 and current_bid > 0):
            break

        position = (position + 1) % 4

    return bids

def main():
    # Read batch request
    with open(r'G:\BBA-CLI\integration-tests\batch-request.json', 'r') as f:
        data = json.load(f)

    ns_conv = data['ns_conventions']
    ew_conv = data['ew_conventions']

    results = []
    for i, deal in enumerate(data['deals']):
        try:
            auction = run_auction(
                deal['pbn'],
                deal['dealer'],
                deal['vulnerability'],
                ns_conv,
                ew_conv
            )
            results.append({
                'deal': deal['pbn'],
                'auction': auction,
                'success': True,
                'error': None
            })
        except Exception as e:
            results.append({
                'deal': deal['pbn'],
                'auction': None,
                'success': False,
                'error': str(e)
            })

        if (i + 1) % 50 == 0:
            print(f"Processed {i + 1} deals...", file=sys.stderr)

    # Output results
    output = {'results': results}
    print(json.dumps(output))

if __name__ == '__main__':
    main()
