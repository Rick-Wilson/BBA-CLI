# Auction Differences: Reference vs Our Implementation

## Summary

Out of ~500 deals in 1N.pbn, approximately 10% show different auctions between the reference BBA.exe output and our epbot-wrapper/rust implementation.

## First 15 Differences

| Board | North Hand | Ref Auction | Our Auction | Diff |
|-------|------------|-------------|-------------|------|
| 2 | J754.K63.QJ96.AT | 1NT 2C X Pass 2H... | 1NT 2C 3NT... | pos 2: X vs 3NT |
| 18 | 965.KJT.T65.KJ94 | 1NT Pass Pass Pass | 1NT Pass 3NT... | pos 2: Pass vs 3NT |
| 38 | K9874.KQT4.32.72 | 1NT Pass 2C... | 1NT Pass 2H... | pos 2: 2C vs 2H |
| 43 | J6.K863.AT75.764 | ...3H Pass 4H... | ...3H Pass Pass... | pos 8: 4H vs Pass |
| 52 | 62.AK85.AJ762.J6 | ...2D Pass 3D... | ...2D Pass 3NT... | pos 6: 3D vs 3NT |
| 53 | A32.J872.T5.KT54 | ...2C Pass 2H... | ...2C 2D... | pos 3: Pass vs 2D (EW bid) |
| 61 | K84.QT85.T92.Q42 | 1NT Pass 2C... | 1NT Pass 2NT... | pos 2: 2C vs 2NT |
| 65 | T643.KQ.J6.KJ764 | ...2C 2D 2H... | ...2C 2D Pass... | pos 4: 2H vs Pass |
| 73 | T875.KJ8753..J84 | ...2NT Pass 3H... | ...2NT Pass 4H... | pos 6: 3H vs 4H |
| 75 | 7.KQ54.Q653.AT85 | ...2D Pass 2NT... | ...2D Pass 3C... | pos 6: 2NT vs 3C |
| 88 | QT3.T632.K82.AT8 | 1NT 3D X... | 1NT 3D 3NT... | pos 2: X vs 3NT |
| 90 | K4.K765.AJ8632.J | ...2S Pass 4D... | ...2S Pass 3D... | pos 6: 4D vs 3D |
| 104 | 5.T652.AKJT872.8 | 1NT 2C X... | 1NT 2C 3C... | pos 2: X vs 3C |
| 115 | AJT4.AK3.T843.93 | ...2D Pass 3NT... | ...2D Pass 3D... | pos 6: 3NT vs 3D |
| 131 | QT75.A52.753.A74 | 1NT Pass 2C... | 1NT Pass 3NT... | pos 2: 2C vs 3NT |

## Observed Patterns

### Pattern 1: Doubles vs Direct Bids (Boards 2, 88, 104)
After 1NT and opponent interference, the reference uses X (double) but we bid directly.
- Board 2: 1NT-2C, ref plays X, we play 3NT
- Board 88: 1NT-3D, ref plays X, we play 3NT
- Board 104: 1NT-2C, ref plays X, we play 3C

### Pattern 2: Pass vs Game (Board 18)
North passes with 9 HCP in reference but we bid 3NT.

### Pattern 3: Stayman vs Other Bids (Boards 38, 61, 131)
Responder uses Stayman (2C) in reference but we bid differently:
- Board 38: ref 2C, we 2H
- Board 61: ref 2C, we 2NT
- Board 131: ref 2C, we 3NT

### Pattern 4: Level Differences in Continuations
- Board 43: ref bids 4H, we pass
- Board 73: ref bids 3H, we bid 4H
- Board 90: ref bids 4D, we bid 3D
- Board 115: ref bids 3NT, we bid 3D

## Hypothesis

The differences may be related to how we're encoding/decoding bids or how the convention card is being loaded. The systematic nature of some differences (especially the X vs direct bid pattern) suggests a configuration or API usage issue rather than random errors.
