using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Text.Json;

namespace EPBotWrapper
{
    /// <summary>
    /// Command-line wrapper for EPBot .NET DLL.
    /// Communicates via JSON on stdin/stdout for easy integration with Rust.
    ///
    /// Usage:
    ///   epbot-wrapper.exe < input.json > output.json
    ///
    /// Or interactive mode (one JSON object per line):
    ///   epbot-wrapper.exe --interactive
    /// </summary>
    class Program
    {
        // Use dynamic to allow loading either EPBot64 or EPBotARM64 at runtime
        private static dynamic CreateEPBot()
        {
            // Try to load the appropriate EPBot assembly
            string baseDir = AppDomain.CurrentDomain.BaseDirectory;

            // Try EPBot64 first (x64), then EPBotARM64
            string[] dllNames = { "EPBot64.dll", "EPBotARM64.dll" };

            foreach (var dllName in dllNames)
            {
                string dllPath = Path.Combine(baseDir, dllName);
                if (File.Exists(dllPath))
                {
                    try
                    {
                        var assembly = Assembly.LoadFrom(dllPath);
                        var typeName = dllName.Replace(".dll", "") + ".EPBot";
                        var type = assembly.GetType(typeName);
                        if (type != null)
                        {
                            return Activator.CreateInstance(type);
                        }
                    }
                    catch
                    {
                        // Try next DLL
                    }
                }
            }

            throw new Exception("Could not load EPBot DLL. Ensure EPBot64.dll or EPBotARM64.dll is in the same directory.");
        }

        static int Main(string[] args)
        {
            try
            {
                bool interactive = args.Length > 0 && args[0] == "--interactive";
                bool testMode = args.Length > 0 && args[0] == "--test";

                if (testMode)
                {
                    return RunTest();
                }
                else if (interactive)
                {
                    return RunInteractive();
                }
                else
                {
                    return RunBatch();
                }
            }
            catch (Exception ex)
            {
                var error = new { error = ex.Message, stackTrace = ex.StackTrace };
                Console.WriteLine(JsonSerializer.Serialize(error));
                return 1;
            }
        }

        /// <summary>
        /// Test mode: Just verify EPBot can be loaded and create an auction
        /// </summary>
        static int RunTest()
        {
            Console.Error.WriteLine("EPBot Wrapper Test Mode");

            try
            {
                dynamic bot = CreateEPBot();
                Console.Error.WriteLine($"EPBot version: {bot.version()}");

                // Try to set up a simple hand
                string[] northHand = new string[] { "AK32", "KQ5", "AQ4", "K87" };  // 17 HCP balanced

                // Initialize the hand
                bot.new_hand(0, ref northHand, 0, 0, false, false);

                Console.Error.WriteLine("Hand initialized successfully");

                // Try to get a bid
                int bidCode = bot.get_bid();
                Console.Error.WriteLine($"Bid code: {bidCode}");

                // Get bidding as string
                string bidding = bot.get_str_bidding();
                Console.Error.WriteLine($"Bidding: {bidding}");

                Console.WriteLine("{\"success\": true, \"version\": " + bot.version() + "}");
                return 0;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error: {ex.Message}");
                Console.Error.WriteLine($"Stack: {ex.StackTrace}");
                Console.WriteLine("{\"success\": false, \"error\": \"" + ex.Message.Replace("\"", "\\\"") + "\"}");
                return 1;
            }
        }

        /// <summary>
        /// Batch mode: Read entire JSON input, process all deals, output JSON result
        /// </summary>
        static int RunBatch()
        {
            string input = Console.In.ReadToEnd();
            var request = JsonSerializer.Deserialize<BatchRequest>(input);

            if (request == null)
            {
                Console.WriteLine(JsonSerializer.Serialize(new { error = "Invalid JSON input" }));
                return 1;
            }

            // Create EPBot instance
            dynamic bot = CreateEPBot();

            // Load conventions if specified
            if (!string.IsNullOrEmpty(request.ns_conventions) && File.Exists(request.ns_conventions))
            {
                LoadConventions(bot, request.ns_conventions, 0); // 0 = NS
            }
            if (!string.IsNullOrEmpty(request.ew_conventions) && File.Exists(request.ew_conventions))
            {
                LoadConventions(bot, request.ew_conventions, 1); // 1 = EW
            }

            var results = new List<DealResult>();

            foreach (var deal in request.deals)
            {
                var result = ProcessDeal(bot, deal);
                results.Add(result);
            }

            var response = new BatchResponse { results = results };
            Console.WriteLine(JsonSerializer.Serialize(response, new JsonSerializerOptions { WriteIndented = false }));

            return 0;
        }

        /// <summary>
        /// Interactive mode: Process one deal per line (for debugging)
        /// </summary>
        static int RunInteractive()
        {
            Console.Error.WriteLine("EPBot Wrapper - Interactive Mode");
            Console.Error.WriteLine("Enter JSON requests, one per line. Ctrl+C to exit.");

            dynamic bot = CreateEPBot();

            string line;
            while ((line = Console.ReadLine()) != null)
            {
                if (string.IsNullOrWhiteSpace(line)) continue;

                try
                {
                    var request = JsonSerializer.Deserialize<SingleRequest>(line);
                    if (request != null)
                    {
                        if (!string.IsNullOrEmpty(request.ns_conventions) && File.Exists(request.ns_conventions))
                        {
                            LoadConventions(bot, request.ns_conventions, 0);
                        }
                        if (!string.IsNullOrEmpty(request.ew_conventions) && File.Exists(request.ew_conventions))
                        {
                            LoadConventions(bot, request.ew_conventions, 1);
                        }

                        var result = ProcessDeal(bot, request.deal);
                        Console.WriteLine(JsonSerializer.Serialize(result));
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine(JsonSerializer.Serialize(new { error = ex.Message }));
                }
            }

            return 0;
        }

        /// <summary>
        /// Process a single deal and generate its auction.
        /// Uses 4 EPBot instances (one per player) as per Edward's design.
        /// After each bid, broadcast it to all players via set_bid.
        /// </summary>
        static DealResult ProcessDeal(dynamic bot, DealInput deal)
        {
            var result = new DealResult { deal = deal.pbn };

            try
            {
                // Parse the PBN deal string
                var (firstSeat, hands) = ParsePbnDeal(deal.pbn);

                // Set dealer (0=N, 1=E, 2=S, 3=W)
                int dealer = ParseDealer(deal.dealer);

                // Set vulnerability (0=None, 1=NS, 2=EW, 3=Both)
                int vul = ParseVulnerability(deal.vulnerability);

                // Create 4 EPBot instances, one per player
                dynamic[] players = new dynamic[4];
                for (int i = 0; i < 4; i++)
                {
                    players[i] = CreateEPBot();
                    string[] hand = hands[i];
                    players[i].new_hand(i, ref hand, dealer, vul, false, false);
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

                    // Broadcast this bid to ALL players using round as the bid index
                    for (int i = 0; i < 4; i++)
                    {
                        players[i].set_bid(round, bidCode, "");
                    }

                    bids.Add(bidStr);

                    // Track passes for auction end detection
                    if (bidStr == "Pass" || bidStr == "P")
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

                    // Move to next position
                    currentPos = (currentPos + 1) % 4;
                }

                result.auction = bids;
                result.success = true;
            }
            catch (Exception ex)
            {
                result.success = false;
                result.error = ex.Message;
            }

            return result;
        }

        /// <summary>
        /// Parse PBN deal format: "N:S.H.D.C S.H.D.C S.H.D.C S.H.D.C"
        /// Returns (first seat, array of hands where index 0=N, 1=E, 2=S, 3=W)
        /// </summary>
        static (int firstSeat, string[][] hands) ParsePbnDeal(string pbn)
        {
            int colonPos = pbn.IndexOf(':');
            if (colonPos < 0)
                throw new ArgumentException("Invalid PBN deal format - missing colon");

            char firstSeatChar = pbn[colonPos - 1];
            int firstSeat = ParseDealer(firstSeatChar.ToString());

            string handsStr = pbn.Substring(colonPos + 1).Trim();
            string[] handParts = handsStr.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);

            if (handParts.Length != 4)
                throw new ArgumentException($"Invalid PBN deal format - expected 4 hands, got {handParts.Length}");

            string[][] hands = new string[4][];

            for (int i = 0; i < 4; i++)
            {
                int pos = (firstSeat + i) % 4;
                string[] suits = handParts[i].Split('.');

                if (suits.Length != 4)
                    throw new ArgumentException($"Invalid hand format - expected 4 suits, got {suits.Length}");

                hands[pos] = suits;
            }

            return (firstSeat, hands);
        }

        /// <summary>
        /// Decode bid code to string
        /// </summary>
        static string DecodeBid(int code)
        {
            if (code == 0) return "Pass";
            if (code == 36) return "X";
            if (code == 37) return "XX";

            if (code >= 1 && code <= 35)
            {
                int level = (code - 1) / 5 + 1;
                int suit = (code - 1) % 5;
                string[] suits = { "C", "D", "H", "S", "NT" };
                return $"{level}{suits[suit]}";
            }

            return "Pass";
        }

        /// <summary>
        /// Load conventions from a .bbsa file
        /// </summary>
        static void LoadConventions(dynamic bot, string filePath, int side)
        {
            foreach (var line in File.ReadAllLines(filePath))
            {
                if (string.IsNullOrWhiteSpace(line) || line.StartsWith("#") || line.StartsWith(";"))
                    continue;

                var parts = line.Split(new[] { '=' }, 2);
                if (parts.Length != 2) continue;

                string key = parts[0].Trim();
                string valueStr = parts[1].Trim();

                if (valueStr == "1" || valueStr.ToLower() == "true")
                {
                    try { bot.set_conventions(side, key, true); } catch { }
                }
                else if (valueStr == "0" || valueStr.ToLower() == "false")
                {
                    try { bot.set_conventions(side, key, false); } catch { }
                }
            }
        }

        static int ParseDealer(string dealer)
        {
            if (string.IsNullOrEmpty(dealer)) return 0;
            switch (dealer.ToUpper())
            {
                case "N": case "NORTH": return 0;
                case "E": case "EAST": return 1;
                case "S": case "SOUTH": return 2;
                case "W": case "WEST": return 3;
                default: return 0;
            }
        }

        static int ParseVulnerability(string vul)
        {
            if (string.IsNullOrEmpty(vul)) return 0;
            switch (vul.ToUpper().Replace("-", ""))
            {
                case "NONE": case "": case "-": return 0;
                case "NS": case "NORTHSOUTH": return 1;
                case "EW": case "EASTWEST": return 2;
                case "BOTH": case "ALL": return 3;
                default: return 0;
            }
        }
    }

    // JSON request/response types

    public class BatchRequest
    {
        public string ns_conventions { get; set; }
        public string ew_conventions { get; set; }
        public List<DealInput> deals { get; set; }
    }

    public class SingleRequest
    {
        public string ns_conventions { get; set; }
        public string ew_conventions { get; set; }
        public DealInput deal { get; set; }
    }

    public class DealInput
    {
        public string pbn { get; set; }
        public string dealer { get; set; }
        public string vulnerability { get; set; }
    }

    public class DealResult
    {
        public string deal { get; set; }
        public List<string> auction { get; set; }
        public bool success { get; set; }
        public string error { get; set; }
    }

    public class BatchResponse
    {
        public List<DealResult> results { get; set; }
    }
}
