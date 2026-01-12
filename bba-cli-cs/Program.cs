using System;
using System.Collections.Generic;
using System.IO;

namespace BbaCli
{
    /// <summary>
    /// BBA CLI - Bridge Bidding Analyzer Command Line Interface
    /// Directly uses EPBot64.dll for auction generation (no subprocess overhead).
    /// </summary>
    class Program
    {
        const string Version = "0.3.0";

        static int Main(string[] args)
        {
            try
            {
                string inputPath = null;
                string outputPath = null;
                string nsConventions = null;
                string ewConventions = null;
                bool verbose = false;
                bool dryRun = false;

                // Parse arguments (matching Rust CLI)
                for (int i = 0; i < args.Length; i++)
                {
                    switch (args[i])
                    {
                        case "-i":
                        case "--input":
                            inputPath = args[++i];
                            break;
                        case "-o":
                        case "--output":
                            outputPath = args[++i];
                            break;
                        case "--ns-conventions":
                            nsConventions = args[++i];
                            break;
                        case "--ew-conventions":
                            ewConventions = args[++i];
                            break;
                        case "-v":
                        case "--verbose":
                            verbose = true;
                            break;
                        case "--dry-run":
                            dryRun = true;
                            break;
                        case "-j":
                        case "--threads":
                            i++; // Skip value - not implemented but accept for compatibility
                            break;
                        case "--wrapper":
                            i++; // Skip value - not needed in C# version
                            break;
                        case "-h":
                        case "--help":
                            PrintUsage();
                            return 0;
                        case "--version":
                            Console.WriteLine($"bba-cli {Version}");
                            return 0;
                    }
                }

                // Validate required arguments
                if (string.IsNullOrEmpty(inputPath))
                {
                    Console.Error.WriteLine("Error: --input is required");
                    PrintUsage();
                    return 1;
                }
                if (string.IsNullOrEmpty(outputPath))
                {
                    Console.Error.WriteLine("Error: --output is required");
                    PrintUsage();
                    return 1;
                }
                if (string.IsNullOrEmpty(nsConventions))
                {
                    Console.Error.WriteLine("Error: --ns-conventions is required");
                    PrintUsage();
                    return 1;
                }
                if (string.IsNullOrEmpty(ewConventions))
                {
                    Console.Error.WriteLine("Error: --ew-conventions is required");
                    PrintUsage();
                    return 1;
                }

                // Validate files exist
                if (!File.Exists(inputPath))
                {
                    Console.Error.WriteLine($"Error: Input file not found: {inputPath}");
                    return 1;
                }
                if (!File.Exists(nsConventions))
                {
                    Console.Error.WriteLine($"Error: NS conventions file not found: {nsConventions}");
                    return 1;
                }
                if (!File.Exists(ewConventions))
                {
                    Console.Error.WriteLine($"Error: EW conventions file not found: {ewConventions}");
                    return 1;
                }

                if (verbose)
                {
                    Console.Error.WriteLine($"BBA-CLI v{Version}");
                    Console.Error.WriteLine($"Input: {inputPath}");
                    Console.Error.WriteLine($"Output: {outputPath}");
                    Console.Error.WriteLine($"NS Conventions: {nsConventions}");
                    Console.Error.WriteLine($"EW Conventions: {ewConventions}");
                }

                var processor = new PbnProcessor { Verbose = verbose };
                var stats = processor.ProcessFile(inputPath, outputPath, nsConventions, ewConventions, dryRun);

                Console.Error.WriteLine($"Processed {stats.DealsProcessed} deals, generated {stats.AuctionsGenerated} auctions");
                if (stats.Errors > 0)
                {
                    Console.Error.WriteLine($"{stats.Errors} deals had errors");
                }

                if (dryRun)
                {
                    Console.Error.WriteLine("Dry run complete - no output written");
                }
                else if (verbose)
                {
                    Console.Error.WriteLine($"Output written to {outputPath}");
                }

                return stats.Errors > 0 ? 1 : 0;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error: {ex.Message}");
                return 1;
            }
        }

        static void PrintUsage()
        {
            Console.WriteLine($"BBA-CLI v{Version} - Bridge Bidding Analyzer");
            Console.WriteLine();
            Console.WriteLine("Generates bridge auctions for deals in PBN files using the EPBot engine.");
            Console.WriteLine();
            Console.WriteLine("Usage: bba-cli --input <FILE> --output <FILE> --ns-conventions <FILE> --ew-conventions <FILE>");
            Console.WriteLine();
            Console.WriteLine("Options:");
            Console.WriteLine("  -i, --input <FILE>           Input PBN file containing deals to analyze");
            Console.WriteLine("  -o, --output <FILE>          Output PBN file for results with generated auctions");
            Console.WriteLine("  --ns-conventions <FILE>      Convention file (.bbsa) for North-South partnership");
            Console.WriteLine("  --ew-conventions <FILE>      Convention file (.bbsa) for East-West partnership");
            Console.WriteLine("  -v, --verbose                Enable verbose logging");
            Console.WriteLine("  --dry-run                    Parse input but don't write output");
            Console.WriteLine("  -j, --threads <N>            (ignored, for compatibility)");
            Console.WriteLine("  --wrapper <FILE>             (ignored, for compatibility)");
            Console.WriteLine("  -h, --help                   Show this help");
            Console.WriteLine("  --version                    Show version");
        }
    }

    public class ProcessingStats
    {
        public int DealsProcessed { get; set; }
        public int AuctionsGenerated { get; set; }
        public int Errors { get; set; }
    }

    /// <summary>
    /// Processes PBN files using EPBot64 directly (no subprocess).
    /// </summary>
    public class PbnProcessor
    {
        private const int C_NORTH = 0;
        private const int C_EAST = 1;
        private const int C_SOUTH = 2;
        private const int C_WEST = 3;

        public bool Verbose { get; set; }

        public ProcessingStats ProcessFile(string inputPath, string outputPath, string nsConventions, string ewConventions, bool dryRun = false)
        {
            var stats = new ProcessingStats();
            var games = ReadPbnFile(inputPath);

            if (Verbose)
                Console.Error.WriteLine($"Read {games.Count} games from {inputPath}");

            // Process each game
            foreach (var game in games)
            {
                if (game.Deal == null) continue;

                stats.DealsProcessed++;

                try
                {
                    var auction = GenerateAuction(game, nsConventions, ewConventions);
                    game.Auction = auction;
                    stats.AuctionsGenerated++;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"Error processing game {stats.DealsProcessed}: {ex.Message}");
                    stats.Errors++;
                }
            }

            // Write output
            if (!dryRun)
            {
                WritePbnFile(outputPath, games);
                if (Verbose)
                    Console.Error.WriteLine($"Wrote {games.Count} games to {outputPath}");
            }

            return stats;
        }

        /// <summary>
        /// Generate auction for a single game using 4 EPBot instances.
        /// </summary>
        private List<string> GenerateAuction(PbnGame game, string nsConventions, string ewConventions)
        {
            int dealer = game.Dealer;
            int vul = game.Vulnerability;
            string[][] hands = game.Deal;

            // Create 4 EPBot instances (direct reference, no reflection!)
            EPBot64.EPBot[] players = new EPBot64.EPBot[4];

            for (int i = 0; i < 4; i++)
            {
                players[i] = new EPBot64.EPBot();

                // Set up the hand
                string[] hand = hands[i];
                players[i].new_hand(i, ref hand, dealer, vul, false, false);

                // Set scoring mode (1 = IMP)
                players[i].scoring = 1;

                // Load conventions
                if (!string.IsNullOrEmpty(nsConventions) && File.Exists(nsConventions))
                {
                    LoadConventions(players[i], nsConventions, 0);
                }
                if (!string.IsNullOrEmpty(ewConventions) && File.Exists(ewConventions))
                {
                    LoadConventions(players[i], ewConventions, 1);
                }
            }

            // Generate auction
            var bids = new List<string>();
            int currentPos = dealer;
            int passCount = 0;
            bool hasBid = false;

            for (int round = 0; round < 100; round++) // Safety limit
            {
                // Get bid from current player
                int bidCode = players[currentPos].get_bid();
                string bidStr = DecodeBid(bidCode);
                bids.Add(bidStr);

                // Broadcast to all players
                for (int i = 0; i < 4; i++)
                {
                    players[i].set_bid(currentPos, bidCode);
                }

                // Track passes
                if (bidStr == "Pass")
                {
                    passCount++;
                }
                else
                {
                    passCount = 0;
                    hasBid = true;
                }

                // Auction ends with 3 passes after a bid, or 4 initial passes
                if ((hasBid && passCount >= 3) || (!hasBid && passCount >= 4))
                {
                    break;
                }

                currentPos = (currentPos + 1) % 4;
            }

            return bids;
        }

        /// <summary>
        /// Load conventions from a .bbsa file.
        /// </summary>
        private void LoadConventions(EPBot64.EPBot bot, string filePath, int side)
        {
            foreach (var line in File.ReadAllLines(filePath))
            {
                if (string.IsNullOrWhiteSpace(line) || line.StartsWith("#") || line.StartsWith(";"))
                    continue;

                var parts = line.Split(new[] { '=' }, 2);
                if (parts.Length != 2) continue;

                string key = parts[0].Trim();
                string valueStr = parts[1].Trim();

                if (int.TryParse(valueStr, out int intValue))
                {
                    if (key.Equals("System type", StringComparison.OrdinalIgnoreCase))
                    {
                        bot.set_system_type(side, intValue);
                    }
                    else if (key.Equals("Opponent type", StringComparison.OrdinalIgnoreCase))
                    {
                        bot.set_opponent_type(side, intValue);
                    }
                }
                else if (bool.TryParse(valueStr, out bool boolValue))
                {
                    try
                    {
                        bot.set_conventions(side, key, boolValue);
                    }
                    catch { /* Ignore unknown conventions */ }
                }
            }
        }

        /// <summary>
        /// Decode EPBot bid code to string.
        /// </summary>
        private string DecodeBid(int code)
        {
            if (code == 0) return "Pass";
            if (code == 1) return "X";
            if (code == 2) return "XX";

            if (code >= 5 && code <= 39)
            {
                int adjustedCode = code - 5;
                int level = adjustedCode / 5 + 1;
                int suit = adjustedCode % 5;
                string[] suits = { "C", "D", "H", "S", "NT" };
                return $"{level}{suits[suit]}";
            }

            return $"?{code}";
        }

        #region PBN File I/O

        private List<PbnGame> ReadPbnFile(string path)
        {
            var games = new List<PbnGame>();
            PbnGame current = null;

            foreach (var line in File.ReadAllLines(path))
            {
                string trimmed = line.Trim();

                if (string.IsNullOrEmpty(trimmed))
                {
                    if (current != null && current.Tags.Count > 0)
                    {
                        games.Add(current);
                        current = null;
                    }
                    continue;
                }

                if (trimmed.StartsWith("[") && trimmed.EndsWith("]"))
                {
                    if (current == null)
                        current = new PbnGame();

                    var (name, value) = ParseTag(trimmed);
                    current.Tags.Add((name, value));

                    if (name.Equals("Deal", StringComparison.OrdinalIgnoreCase))
                    {
                        current.Deal = ParseDeal(value, out int firstSeat);
                    }
                    else if (name.Equals("Dealer", StringComparison.OrdinalIgnoreCase))
                    {
                        current.Dealer = ParsePosition(value);
                    }
                    else if (name.Equals("Vulnerable", StringComparison.OrdinalIgnoreCase))
                    {
                        current.Vulnerability = ParseVulnerability(value);
                    }
                }
                else if (current != null)
                {
                    current.OtherLines.Add(line);
                }
            }

            if (current != null && current.Tags.Count > 0)
            {
                games.Add(current);
            }

            return games;
        }

        private void WritePbnFile(string path, List<PbnGame> games)
        {
            using (var writer = new StreamWriter(path))
            {
                for (int i = 0; i < games.Count; i++)
                {
                    if (i > 0) writer.WriteLine();

                    var game = games[i];

                    // Write tags
                    foreach (var (name, value) in game.Tags)
                    {
                        writer.WriteLine($"[{name} \"{value}\"]");
                    }

                    // Write auction if generated
                    if (game.Auction != null && game.Auction.Count > 0)
                    {
                        string dealerChar = new[] { "N", "E", "S", "W" }[game.Dealer];
                        writer.WriteLine($"[Auction \"{dealerChar}\"]");
                        writer.WriteLine(FormatAuction(game.Auction));
                    }

                    // Write other lines
                    foreach (var line in game.OtherLines)
                    {
                        writer.WriteLine(line);
                    }
                }
            }
        }

        private (string name, string value) ParseTag(string line)
        {
            string inner = line.TrimStart('[').TrimEnd(']');
            int space = inner.IndexOf(' ');
            if (space < 0) return (inner, "");

            string name = inner.Substring(0, space);
            string value = inner.Substring(space + 1).Trim('"');
            return (name, value);
        }

        /// <summary>
        /// Parse PBN deal. Returns hands[4][4] where hands[position][suit].
        /// PBN format is S.H.D.C but EPBot expects C.D.H.S, so we reverse.
        /// </summary>
        private string[][] ParseDeal(string deal, out int firstSeat)
        {
            int colonPos = deal.IndexOf(':');
            firstSeat = ParsePosition(deal.Substring(colonPos - 1, 1));

            string handsStr = deal.Substring(colonPos + 1).Trim();
            string[] handParts = handsStr.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);

            string[][] hands = new string[4][];
            for (int i = 0; i < 4; i++)
            {
                int pos = (firstSeat + i) % 4;
                string[] suits = handParts[i].Split('.');
                Array.Reverse(suits); // PBN S.H.D.C -> EPBot C.D.H.S
                hands[pos] = suits;
            }

            return hands;
        }

        private int ParsePosition(string s)
        {
            switch (s.Trim().ToUpper())
            {
                case "N": return C_NORTH;
                case "E": return C_EAST;
                case "S": return C_SOUTH;
                case "W": return C_WEST;
                default: return C_NORTH;
            }
        }

        private int ParseVulnerability(string s)
        {
            switch (s.Trim().ToUpper())
            {
                case "NONE":
                case "-":
                    return 0;
                case "NS":
                case "N-S":
                    return 2;
                case "EW":
                case "E-W":
                    return 1;
                case "BOTH":
                case "ALL":
                    return 3;
                default:
                    return 0;
            }
        }

        private string FormatAuction(List<string> bids)
        {
            var lines = new List<string>();
            for (int i = 0; i < bids.Count; i += 4)
            {
                int count = Math.Min(4, bids.Count - i);
                lines.Add(string.Join(" ", bids.GetRange(i, count)));
            }
            return string.Join("\n", lines);
        }

        #endregion
    }

    public class PbnGame
    {
        public List<(string Name, string Value)> Tags { get; } = new List<(string, string)>();
        public List<string> OtherLines { get; } = new List<string>();
        public string[][] Deal { get; set; }
        public int Dealer { get; set; }
        public int Vulnerability { get; set; }
        public List<string> Auction { get; set; }
    }
}
