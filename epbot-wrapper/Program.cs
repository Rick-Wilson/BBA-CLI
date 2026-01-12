using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
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
                Console.Error.WriteLine($"Trying {dllPath} - exists: {File.Exists(dllPath)}");
                if (File.Exists(dllPath))
                {
                    try
                    {
                        var assembly = Assembly.LoadFrom(dllPath);
                        Console.Error.WriteLine($"Loaded assembly: {assembly.FullName}");
                        var typeName = dllName.Replace(".dll", "") + ".EPBot";
                        Console.Error.WriteLine($"Looking for type: {typeName}");
                        var type = assembly.GetType(typeName);
                        Console.Error.WriteLine($"Found type: {type}");
                        if (type == null)
                        {
                            Console.Error.WriteLine("Available types:");
                            foreach (var t in assembly.GetTypes())
                                Console.Error.WriteLine($"  {t.FullName}");
                        }
                        if (type != null)
                        {
                            return Activator.CreateInstance(type);
                        }
                    }
                    catch (Exception ex)
                    {
                        Console.Error.WriteLine($"Error loading: {ex.Message}");
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
                bool convMode = args.Length > 0 && args[0] == "--conventions";

                if (convMode)
                {
                    string convFile = args.Length > 1 ? args[1] : @"P:\bbsa\21GF-DEFAULT.bbsa";
                    return DumpConventions(convFile);
                }
                else if (testMode)
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
        /// Dump all conventions loaded from a .bbsa file
        /// </summary>
        static int DumpConventions(string convFile)
        {
            Console.WriteLine($"Loading conventions from: {convFile}");

            dynamic bot = CreateEPBot();
            bot.scoring = 0;

            // Load conventions for NS (side 0) and EW (side 1)
            LoadConventions(bot, convFile, 0);
            LoadConventions(bot, convFile, 1);

            // Get system type
            int sysType = bot.get_system_type(0);
            int oppType = bot.get_opponent_type(0);
            Console.WriteLine($"System type: {sysType}");
            Console.WriteLine($"Opponent type: {oppType}");

            // Get selected conventions
            Console.WriteLine("\n=== Selected Conventions (enabled) ===");
            try
            {
                string[] selected = bot.selected_conventions();
                Console.WriteLine($"  (Total: {selected.Length} entries)");
                // Only show non-empty entries with differences
                foreach (var c in selected)
                {
                    if (!string.IsNullOrWhiteSpace(c) && c.Contains("True") && c.Contains("False"))
                    {
                        Console.WriteLine($"  DIFF: {c}");
                    }
                }
                Console.WriteLine("\n  (Full list where both True:)");
                foreach (var c in selected)
                {
                    if (!string.IsNullOrWhiteSpace(c) && c.EndsWith("True True"))
                    {
                        Console.WriteLine($"  {c}");
                    }
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error getting selected_conventions: {ex.Message}");
            }

            // Query specific conventions mentioned in reference file
            Console.WriteLine("\n=== Key Conventions Check ===");
            string[] keyConventions = {
                "Lebensohl after 1NT",
                "Cappelletti",
                "Stayman",
                "Jacoby 2NT",
                "Texas",
                "New Minor Forcing",
                "Fourth suit game force",
                "Support double redouble",
                "Unusual 2NT",
                "Michaels Cuebid",
                "Weak Jump Shifts 3",
                "Blackwood 0314",
                "Gerber",
                "1N-2S transfer to clubs",
                "1N-3C transfer to diamonds"
            };

            foreach (var conv in keyConventions)
            {
                try
                {
                    bool enabled = bot.get_conventions(0, conv);
                    Console.WriteLine($"  {conv} = {enabled}");
                }
                catch
                {
                    Console.WriteLine($"  {conv} = [not found]");
                }
            }

            // Also dump all conventions from the file with their loaded values
            Console.WriteLine("\n=== All Conventions from File ===");
            foreach (var line in File.ReadAllLines(convFile))
            {
                if (string.IsNullOrWhiteSpace(line) || line.StartsWith("#") || line.StartsWith(";"))
                    continue;

                var parts = line.Split(new[] { '=' }, 2);
                if (parts.Length != 2) continue;

                string key = parts[0].Trim();
                string fileVal = parts[1].Trim();

                // Skip system/opponent type
                if (key == "System type" || key == "Opponent type") continue;

                try
                {
                    bool loaded = bot.get_conventions(0, key);
                    string match = (fileVal == "1" && loaded) || (fileVal == "0" && !loaded) ? "" : " <-- MISMATCH";
                    Console.WriteLine($"  {key}: file={fileVal}, loaded={loaded}{match}");
                }
                catch
                {
                    // Convention name not recognized by EPBot
                }
            }

            return 0;
        }

        /// <summary>
        /// Test mode: List all EPBot methods and test bidding
        /// </summary>
        static int RunTest()
        {
            Console.Error.WriteLine("EPBot Wrapper Test Mode");

            try
            {
                dynamic bot = CreateEPBot();

                // Get the type and list methods
                Type botType = bot.GetType();
                Console.Error.WriteLine($"Type: {botType.FullName}");

                // Get EPBot internal version
                int epbotVersion = bot.version();
                Console.Error.WriteLine($"EPBot internal version: {epbotVersion}");

                // List all methods
                Console.Error.WriteLine("\n=== All EPBot Methods ===");
                foreach (var method in botType.GetMethods(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly))
                {
                    var parms = string.Join(", ", method.GetParameters().Select(p => $"{p.ParameterType.Name} {p.Name}"));
                    Console.Error.WriteLine($"  {method.ReturnType.Name} {method.Name}({parms})");
                }

                // Test Board 2 from 1m-1N.pbn - should open 1C
                // Deal: N:A653.Q97.K64.954 KQ4.AT8432.A72.A J8.K.QJ98.KJT762 T972.J65.T53.Q83
                // Dealer: S, Vul: NS
                // South has J8.K.QJ98.KJT762 = 2-1-4-6 shape, should open 1C

                Console.Error.WriteLine("\n=== Testing Board 2 from 1m-1N.pbn ===");
                Console.Error.WriteLine("Deal: N:A653.Q97.K64.954 KQ4.AT8432.A72.A J8.K.QJ98.KJT762 T972.J65.T53.Q83");
                Console.Error.WriteLine("Expected: 1C (South opens with 6 clubs)");

                // Create 4 players - Board 2 from 1m-1N.pbn
                // N:A653.Q97.K64.954 KQ4.AT8432.A72.A J8.K.QJ98.KJT762 T972.J65.T53.Q83
                // PBN format is S.H.D.C but EPBot might expect C.D.H.S - let's try reversed!
                string[][] hands = new string[4][];
                hands[0] = new[] { "954", "K64", "Q97", "A653" };        // N (reversed: C.D.H.S)
                hands[1] = new[] { "A", "A72", "AT8432", "KQ4" };        // E (reversed: C.D.H.S)
                hands[2] = new[] { "KJT762", "QJ98", "K", "J8" };        // S (reversed: should open 1C now)
                hands[3] = new[] { "Q83", "T53", "J65", "T972" };        // W (reversed: C.D.H.S)

                int dealer = 2; // South
                int vul = 1;    // NS

                // Create players
                dynamic[] players = new dynamic[4];
                string[] posNames = { "N", "E", "S", "W" };

                for (int i = 0; i < 4; i++)
                {
                    players[i] = CreateEPBot();
                    players[i].scoring = 0;  // Set scoring like Edward's code
                    // No conventions needed for pass-out hands
                    string[] hand = hands[i];
                    players[i].new_hand(i, ref hand, dealer, vul, false, false);

                    // Debug: print what EPBot thinks the hand is AND what position it thinks it's in
                    int epbotPos = players[i].get_Position();
                    string[] epbotHand = players[i].get_hand(i);
                    Console.Error.WriteLine($"{posNames[i]} (pos {i}): input={string.Join(".", hands[i])} -> EPBot pos={epbotPos}, sees: {string.Join(".", epbotHand)}");

                    // Also check what hand EPBot has at each position
                    for (int j = 0; j < 4; j++)
                    {
                        string[] h = players[i].get_hand(j);
                        Console.Error.WriteLine($"    get_hand({j}) = {string.Join(".", h)}");
                    }
                }

                // Test get_bid vs ask
                Console.Error.WriteLine("\n=== Comparing get_bid() vs ask() ===");

                int currentPos = dealer;
                var bidsGetBid = new List<string>();
                var bidsAsk = new List<string>();

                // Debug: What does each bot return for get_bid() BEFORE any bidding?
                Console.Error.WriteLine("\n=== Initial get_bid() from each bot (before any bidding) ===");
                for (int i = 0; i < 4; i++)
                {
                    int bidCode = players[i].get_bid();
                    string bidStr = DecodeBid(bidCode);
                    Console.Error.WriteLine($"  Bot for {posNames[i]}: get_bid() = {bidStr} (code {bidCode})");
                }

                // First try with get_bid()
                Console.Error.WriteLine("\nUsing get_bid() in auction order:");
                for (int round = 0; round < 4; round++)
                {
                    int bidCode = players[currentPos].get_bid();
                    string bidStr = DecodeBid(bidCode);
                    bidsGetBid.Add(bidStr);
                    Console.Error.WriteLine($"  {posNames[currentPos]}: {bidStr} (code {bidCode})");

                    // Broadcast to all - use 2-param set_bid like Edward's code
                    for (int j = 0; j < 4; j++)
                        players[j].set_bid(currentPos, bidCode);

                    currentPos = (currentPos + 1) % 4;
                }

                // Now recreate and try with ask()
                Console.Error.WriteLine("\nUsing ask():");
                for (int i = 0; i < 4; i++)
                {
                    players[i] = CreateEPBot();
                    players[i].scoring = 0;
                    string[] hand = hands[i];
                    players[i].new_hand(i, ref hand, dealer, vul, false, false);
                }

                currentPos = dealer;
                for (int round = 0; round < 4; round++)
                {
                    int bidCode = players[currentPos].ask();
                    string bidStr = DecodeBid(bidCode);
                    bidsAsk.Add(bidStr);
                    Console.Error.WriteLine($"  {posNames[currentPos]}: {bidStr} (code {bidCode})");

                    // Broadcast to all - use 2-param set_bid
                    for (int j = 0; j < 4; j++)
                        players[j].set_bid(currentPos, bidCode);

                    currentPos = (currentPos + 1) % 4;
                }

                Console.Error.WriteLine($"\nget_bid() produced: {string.Join(" ", bidsGetBid)}");
                Console.Error.WriteLine($"ask() produced: {string.Join(" ", bidsAsk)}");
                Console.Error.WriteLine("Expected: 1C Pass 1S 2H (South opens 1C with 6 clubs)");

                // Test the problematic deal: after 1NT-2C, what does North bid?
                Console.Error.WriteLine("\n=== Testing 1NT-2C response for North ===");
                Console.Error.WriteLine("Deal: N:J754.K63.QJ96.AT");
                Console.Error.WriteLine("After 1NT-2C, expected: X (double showing values)");

                // Create North bot
                dynamic northBot = CreateEPBot();
                northBot.scoring = 0;  // Set scoring like Edward's code
                // Load conventions for both sides
                LoadConventions(northBot, @"P:\bbsa\21GF-DEFAULT.bbsa", 0);
                LoadConventions(northBot, @"P:\bbsa\21GF-DEFAULT.bbsa", 1);

                // North's hand: J754.K63.QJ96.AT -> in C.D.H.S order: AT.QJ96.K63.J754
                string[] northHand = new[] { "AT", "QJ96", "K63", "J754" };
                northBot.new_hand(0, ref northHand, 2, 2, false, false);  // position=N(0), dealer=S(2), vul=NS(2)

                // Feed the auction: 1NT from South, 2C from West
                // 1NT = code 9, 2C = code 10
                // Use 2-param set_bid like Edward's code
                northBot.set_bid(2, 9);  // South bids 1NT
                northBot.set_bid(3, 10); // West bids 2C

                // Now ask North what it would bid
                int northBid = northBot.get_bid();
                Console.Error.WriteLine($"North bids: {DecodeBid(northBid)} (code {northBid})");
                Console.Error.WriteLine($"Expected: X (code 1)");

                // Also try using set_arr_bids instead
                // Edward's code uses 2-digit format: "09" for 1NT (code 9), "10" for 2C (code 10)
                dynamic northBot2 = CreateEPBot();
                northBot2.scoring = 0;
                LoadConventions(northBot2, @"P:\bbsa\21GF-DEFAULT.bbsa", 0);
                LoadConventions(northBot2, @"P:\bbsa\21GF-DEFAULT.bbsa", 1);
                northBot2.new_hand(0, ref northHand, 2, 2, false, false);

                string[] arrBids = new[] { "09", "10" };  // 1NT=9, 2C=10 in Edward's format
                northBot2.set_arr_bids(ref arrBids);

                int northBid2 = northBot2.get_bid();
                Console.Error.WriteLine($"With set_arr_bids: North bids: {DecodeBid(northBid2)} (code {northBid2})");

                // Try with ask() instead of get_bid()
                dynamic northBot3 = CreateEPBot();
                northBot3.scoring = 0;
                LoadConventions(northBot3, @"P:\bbsa\21GF-DEFAULT.bbsa", 0);
                LoadConventions(northBot3, @"P:\bbsa\21GF-DEFAULT.bbsa", 1);
                northBot3.new_hand(0, ref northHand, 2, 2, false, false);
                northBot3.set_bid(2, 9);  // South bids 1NT
                northBot3.set_bid(3, 10); // West bids 2C

                int northBid3 = northBot3.ask();
                Console.Error.WriteLine($"With ask(): North bids: {DecodeBid(northBid3)} (code {northBid3})");

                // Try with interpret_bid after each set_bid
                dynamic northBot4 = CreateEPBot();
                northBot4.scoring = 0;
                LoadConventions(northBot4, @"P:\bbsa\21GF-DEFAULT.bbsa", 0);
                LoadConventions(northBot4, @"P:\bbsa\21GF-DEFAULT.bbsa", 1);
                northBot4.new_hand(0, ref northHand, 2, 2, false, false);
                northBot4.set_bid(2, 9);
                northBot4.interpret_bid(9);  // Interpret 1NT
                northBot4.set_bid(3, 10);
                northBot4.interpret_bid(10); // Interpret 2C

                int northBid4 = northBot4.get_bid();
                Console.Error.WriteLine($"With interpret_bid: North bids: {DecodeBid(northBid4)} (code {northBid4})");

                Console.WriteLine("{\"success\": true}");
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

            var results = new List<DealResult>();

            foreach (var deal in request.deals)
            {
                var result = ProcessDeal(deal, request.ns_conventions, request.ew_conventions);
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

            string line;
            while ((line = Console.ReadLine()) != null)
            {
                if (string.IsNullOrWhiteSpace(line)) continue;

                try
                {
                    var request = JsonSerializer.Deserialize<SingleRequest>(line);
                    if (request != null)
                    {
                        var result = ProcessDeal(request.deal, request.ns_conventions, request.ew_conventions);
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
        /// Uses 4 EPBot instances per Edward's design.
        /// Pattern: get_bid() from current player, then set_bid(position, bid) to ALL players.
        /// Per API docs: "EPBot.set_bid position, bid - confirm of the player's bid in position"
        /// </summary>
        static DealResult ProcessDeal(DealInput deal, string nsConventions, string ewConventions)
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
                // Each bot needs BOTH partnerships' conventions loaded
                // IMPORTANT: Per Edward's docs, the correct order is:
                //   1. System selection (system_type)
                //   2. Convention setting (set_conventions)
                //   3. new_hand() call
                dynamic[] players = new dynamic[4];
                string[] posNames = { "N", "E", "S", "W" };

                for (int i = 0; i < 4; i++)
                {
                    players[i] = CreateEPBot();

                    // Call new_hand FIRST (matching Edward's VB order)
                    string[] hand = hands[i];
                    players[i].new_hand(i, ref hand, dealer, vul, false, false);

                    // Then set scoring mode (0 = MP, 1 = IMP)
                    players[i].scoring = 1;

                    // Then load conventions
                    // Load NS conventions as side 0 (includes system_type)
                    if (!string.IsNullOrEmpty(nsConventions) && File.Exists(nsConventions))
                    {
                        LoadConventions(players[i], nsConventions, 0);
                    }
                    // Load EW conventions as side 1 (includes system_type)
                    if (!string.IsNullOrEmpty(ewConventions) && File.Exists(ewConventions))
                    {
                        LoadConventions(players[i], ewConventions, 1);
                    }

                    // Debug: print what EPBot thinks the hands are
                    string[] epbotHand = players[i].get_hand(i);
                    Console.Error.WriteLine($"{posNames[i]} hand input: {string.Join(".", hand)} -> EPBot: {string.Join(".", epbotHand)}");

                    // Check key conventions and system types - including ones that differ between DEFAULT and GIB
                    bool lebNS = players[i].get_conventions(0, "Lebensohl after 1NT");
                    bool lebEW = players[i].get_conventions(1, "Lebensohl after 1NT");
                    bool cappNS = players[i].get_conventions(0, "Cappelletti");
                    bool cappEW = players[i].get_conventions(1, "Cappelletti");
                    bool gamblingNS = players[i].get_conventions(0, "Gambling");
                    bool gamblingEW = players[i].get_conventions(1, "Gambling");
                    int sysTypeNS = players[i].get_system_type(0);
                    int sysTypeEW = players[i].get_system_type(1);
                    Console.Error.WriteLine($"  {posNames[i]}: LebNS={lebNS}, LebEW={lebEW}, CappNS={cappNS}, CappEW={cappEW}, GamblingNS={gamblingNS}, GamblingEW={gamblingEW}");
                }

                // Generate auction
                var bids = new List<string>();
                int currentPos = dealer;
                int passCount = 0;
                bool hasBid = false;
                string[] positions = { "N", "E", "S", "W" };
                Console.Error.WriteLine($"Starting auction, dealer = {positions[dealer]}");

                for (int round = 0; round < 100; round++) // Safety limit
                {
                    // Get bid from current player
                    int bidCode = players[currentPos].get_bid();
                    string bidStr = DecodeBid(bidCode);
                    bids.Add(bidStr);

                    // Broadcast this bid to ALL players using set_bid(position, bid)
                    // Note: Edward's code uses 2 params, not 3
                    for (int i = 0; i < 4; i++)
                    {
                        players[i].set_bid(currentPos, bidCode);
                    }

                    // Get bid meaning and extended meaning
                    string bidMeaning = "";
                    string extMeaning = "";
                    try { bidMeaning = players[currentPos].get_info_meaning(bidCode); } catch { }
                    try { extMeaning = players[currentPos].get_info_meaning_extended(currentPos); } catch { }
                    Console.Error.WriteLine($"{positions[currentPos]}: {bidStr}");
                    Console.Error.WriteLine($"  meaning: \"{bidMeaning}\"");
                    Console.Error.WriteLine($"  extended: \"{extMeaning}\"");

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
        /// NOTE: PBN uses S.H.D.C order but EPBot expects C.D.H.S order, so we reverse.
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

                // PBN format is S.H.D.C but EPBot expects C.D.H.S - reverse the array
                Array.Reverse(suits);
                hands[pos] = suits;
            }

            return (firstSeat, hands);
        }

        /// <summary>
        /// Decode bid code to string.
        /// EPBot encoding (derived from testing):
        /// 0 = Pass
        /// 1 = X (Double)
        /// 2 = XX (Redouble)
        /// 3, 4 = reserved/unknown
        /// 5-9 = 1C, 1D, 1H, 1S, 1NT
        /// 10-14 = 2C, 2D, 2H, 2S, 2NT
        /// ...
        /// 35-39 = 7C, 7D, 7H, 7S, 7NT
        /// </summary>
        static string DecodeBid(int code)
        {
            if (code == 0) return "Pass";
            if (code == 1) return "X";
            if (code == 2) return "XX";

            // Codes 5-39 represent bids 1C through 7NT
            if (code >= 5 && code <= 39)
            {
                int adjustedCode = code - 5;
                int level = adjustedCode / 5 + 1;
                int suit = adjustedCode % 5;
                string[] suits = { "C", "D", "H", "S", "NT" };
                return $"{level}{suits[suit]}";
            }

            // Unknown codes
            return $"?{code}";
        }

        /// <summary>
        /// Set all conventions from 21GF-DEFAULT.bbsa explicitly
        /// Uses method calls for C# dynamic compatibility
        /// </summary>
        static void SetMinimalConventions(dynamic bot, int side)
        {
            // System type = 0 (T_21GF)
            bot.set_system_type(side, 0);

            // All conventions from 21GF-DEFAULT.bbsa
            bot.set_conventions(side, "1D opening with 4 cards", false);
            bot.set_conventions(side, "1D opening with 5 cards", false);
            bot.set_conventions(side, "1m opening allows 5M", true);
            bot.set_conventions(side, "1M-3M blocking", false);
            bot.set_conventions(side, "1M-3M inviting", true);
            bot.set_conventions(side, "1N-2S Minor Suit Stayman", false);
            bot.set_conventions(side, "1N-2S transfer to clubs", true);
            bot.set_conventions(side, "1N-2N transfer to clubs", false);
            bot.set_conventions(side, "1N-2N transfer to diamonds", false);
            bot.set_conventions(side, "1N-3C transfer to diamonds", true);
            bot.set_conventions(side, "1N-3C Puppet Stayman", false);
            bot.set_conventions(side, "1N-3D majors", false);
            bot.set_conventions(side, "1N-3D minors", false);
            bot.set_conventions(side, "1N-3D natural", true);
            bot.set_conventions(side, "1N-3D splinter", false);
            bot.set_conventions(side, "1N-3M splinter", false);
            bot.set_conventions(side, "1NT opening allows less 1HCP", false);
            bot.set_conventions(side, "1NT opening natural", false);
            bot.set_conventions(side, "1NT opening NT style", true);
            bot.set_conventions(side, "1NT opening range 12-14", false);
            bot.set_conventions(side, "1NT opening range 13-15", false);
            bot.set_conventions(side, "1NT opening range 14-16", false);
            bot.set_conventions(side, "1NT opening range 15-17", true);
            bot.set_conventions(side, "1NT opening shape 4441", false);
            bot.set_conventions(side, "1NT opening shape 5422", true);
            bot.set_conventions(side, "1NT opening shape 5 major", true);
            bot.set_conventions(side, "1NT opening shape 6 minor", false);
            bot.set_conventions(side, "(1X)-1Y-(1Z)-2Z natural", false);
            bot.set_conventions(side, "1X-(Y)-2Z forcing", true);
            bot.set_conventions(side, "1X-(1Y)-2Z strong", false);
            bot.set_conventions(side, "1X-(1Y)-2Z weak", true);
            bot.set_conventions(side, "2N-3C-3N both majors", false);
            bot.set_conventions(side, "2N-3C Puppet Stayman", false);
            bot.set_conventions(side, "4NT opening", true);
            bot.set_conventions(side, "5431 after 1NT", false);
            bot.set_conventions(side, "5NT pick a slam", false);
            bot.set_conventions(side, "Benjamin 2D", false);
            bot.set_conventions(side, "Bergen", false);
            bot.set_conventions(side, "Blackwood 0123", false);
            bot.set_conventions(side, "Blackwood 0314", true);
            bot.set_conventions(side, "Blackwood 1430", false);
            bot.set_conventions(side, "Blackwood without K and Q", false);
            bot.set_conventions(side, "BROMAD", false);
            bot.set_conventions(side, "Cappelletti", true);
            bot.set_conventions(side, "Checkback", false);
            bot.set_conventions(side, "Crosswood 0123", false);
            bot.set_conventions(side, "Crosswood 0314", false);
            bot.set_conventions(side, "Crosswood 1430", false);
            bot.set_conventions(side, "Cue bid", true);
            bot.set_conventions(side, "DEPO", false);
            bot.set_conventions(side, "DOPI", true);
            bot.set_conventions(side, "Drury", false);
            bot.set_conventions(side, "Exclusion", false);
            bot.set_conventions(side, "Extended Stayman", false);
            bot.set_conventions(side, "Extended acceptance after NT", true);
            bot.set_conventions(side, "Flannery", false);
            bot.set_conventions(side, "Fit showing jumps", false);
            bot.set_conventions(side, "Forcing 1NT", true);
            bot.set_conventions(side, "Fourth suit", false);
            bot.set_conventions(side, "Fourth suit game force", true);
            bot.set_conventions(side, "French 2D", false);
            bot.set_conventions(side, "Gambling", true);
            bot.set_conventions(side, "Garbage Stayman", true);
            bot.set_conventions(side, "Gazzilli", false);
            bot.set_conventions(side, "Gerber", true);
            bot.set_conventions(side, "Gerber only for NT openings", false);
            bot.set_conventions(side, "Ghestem", false);
            bot.set_conventions(side, "Imposible 2S", false);
            bot.set_conventions(side, "Inverted count signals", false);
            bot.set_conventions(side, "Inverted minors", true);
            bot.set_conventions(side, "Inviting Jump Shifts", false);
            bot.set_conventions(side, "Jacoby 2NT", true);
            bot.set_conventions(side, "Jordan Truscott 2NT", true);
            bot.set_conventions(side, "Jordan Truscott 2NT defence", false);
            bot.set_conventions(side, "Kickback 0123", false);
            bot.set_conventions(side, "Kickback 0314", false);
            bot.set_conventions(side, "Kickback 1430", false);
            bot.set_conventions(side, "King ask by 5NT", true);
            bot.set_conventions(side, "King ask by 5NT inviting", false);
            bot.set_conventions(side, "King ask by available bid", false);
            bot.set_conventions(side, "Kokish Relay", false);
            bot.set_conventions(side, "Landy", false);
            bot.set_conventions(side, "Lavinthal from void", true);
            bot.set_conventions(side, "Lavinthal on ace", true);
            bot.set_conventions(side, "Lavinthal on trump", false);
            bot.set_conventions(side, "Lavinthal to void", true);
            bot.set_conventions(side, "Leaping Michaels", false);
            bot.set_conventions(side, "Lebensohl after 1NT", true);
            bot.set_conventions(side, "Lebensohl after 1m", true);
            bot.set_conventions(side, "Lebensohl after double", true);
            bot.set_conventions(side, "Major Direct Jump Cuebid Gambling", false);
            bot.set_conventions(side, "Major Direct Jump Cuebid Minor", false);
            bot.set_conventions(side, "Major Direct Jump Cuebid Strong", false);
            bot.set_conventions(side, "Mark on queen", true);
            bot.set_conventions(side, "Mark on king", true);
            bot.set_conventions(side, "Minor Direct Jump Cuebid Gambling", false);
            bot.set_conventions(side, "Minor Direct Jump Cuebid Majors", false);
            bot.set_conventions(side, "Minor Direct Jump Cuebid Preempt", false);
            bot.set_conventions(side, "Maximal Doubles", false);
            bot.set_conventions(side, "Michaels Cuebid", true);
            bot.set_conventions(side, "Mini Splinter", false);
            bot.set_conventions(side, "Minor Suit Slam Try after 2NT", false);
            bot.set_conventions(side, "Minor Suit Stayman after 2NT", false);
            bot.set_conventions(side, "Minor Suit Transfers after 2NT", true);
            bot.set_conventions(side, "Mixed raise", false);
            bot.set_conventions(side, "Multi", false);
            bot.set_conventions(side, "Multi-Landy", false);
            bot.set_conventions(side, "Namyats", false);
            bot.set_conventions(side, "Natural 3N entering style", false);
            bot.set_conventions(side, "New Minor Forcing", true);
            bot.set_conventions(side, "NMF by passed hand", false);
            bot.set_conventions(side, "Non-Leaping Michaels", false);
            bot.set_conventions(side, "Ogust", false);
            bot.set_conventions(side, "Polish two suiters", false);
            bot.set_conventions(side, "Precision 2D", false);
            bot.set_conventions(side, "Quantitative 4NT", true);
            bot.set_conventions(side, "Raptor 1NT", false);
            bot.set_conventions(side, "Responsive double", true);
            bot.set_conventions(side, "Reverse Bergen", false);
            bot.set_conventions(side, "Reverse drury", true);
            bot.set_conventions(side, "Reverse Flannery 2H", false);
            bot.set_conventions(side, "Reverse Flannery 2S", false);
            bot.set_conventions(side, "ROPI", true);
            bot.set_conventions(side, "Rubensohl after 1NT", false);
            bot.set_conventions(side, "Rubensohl after 1m", false);
            bot.set_conventions(side, "Rubensohl after double", false);
            bot.set_conventions(side, "Scrambling 2NT", false);
            bot.set_conventions(side, "Semi forcing 1NT", false);
            bot.set_conventions(side, "Shape Bergen structure", true);
            bot.set_conventions(side, "SMOLEN", true);
            bot.set_conventions(side, "Snapdragon Double", false);
            bot.set_conventions(side, "Soloway Jump Shifts", false);
            bot.set_conventions(side, "Soloway Jump Shifts Extended", false);
            bot.set_conventions(side, "Splinter", true);
            bot.set_conventions(side, "Strength Lawrence structure", false);
            bot.set_conventions(side, "Strong natural 2D", false);
            bot.set_conventions(side, "Strong natural 2M", false);
            bot.set_conventions(side, "Strong jump shifts 2", true);
            bot.set_conventions(side, "Super acceptance after NT", true);
            bot.set_conventions(side, "Support 1NT", false);
            bot.set_conventions(side, "Support double redouble", true);
            bot.set_conventions(side, "Surplus pass", false);
            bot.set_conventions(side, "Texas", true);
            bot.set_conventions(side, "Transfers if RHO passes", false);
            bot.set_conventions(side, "Transfers if RHO doubles", false);
            bot.set_conventions(side, "Transfers if RHO bids clubs", true);
            bot.set_conventions(side, "Two suit takeout double", true);
            bot.set_conventions(side, "Two way game tries", false);
            bot.set_conventions(side, "Two Way New Minor Forcing", false);
            bot.set_conventions(side, "TWNMF by passed hand", false);
            bot.set_conventions(side, "Unusual 1NT", true);
            bot.set_conventions(side, "Unusual 2NT", true);
            bot.set_conventions(side, "Unusual 3NT", false);
            bot.set_conventions(side, "Unusual 4NT", true);
            bot.set_conventions(side, "Weak Jump Shifts 2", false);
            bot.set_conventions(side, "Weak Jump Shifts 3", true);
            bot.set_conventions(side, "Weak natural 2D", true);
            bot.set_conventions(side, "Weak natural 2M", true);
            bot.set_conventions(side, "Walsh style", false);
            bot.set_conventions(side, "Wilkosz", false);

            // Opponent type = 0
            bot.set_opponent_type(side, 0);
        }

        /// <summary>
        /// Load conventions from a .bbsa file
        /// </summary>
        static bool _conventionsLogged = false;
        static void LoadConventions(dynamic bot, string filePath, int side)
        {
            int success = 0, failed = 0;

            foreach (var line in File.ReadAllLines(filePath))
            {
                if (string.IsNullOrWhiteSpace(line) || line.StartsWith("#") || line.StartsWith(";"))
                    continue;

                var parts = line.Split(new[] { '=' }, 2);
                if (parts.Length != 2) continue;

                string key = parts[0].Trim();
                string valueStr = parts[1].Trim();

                // Try to parse as integer first (for settings like "System type = 3")
                if (int.TryParse(valueStr, out int intValue))
                {
                    try
                    {
                        // Special handling for System type - use set_system_type method
                        if (key.Equals("System type", StringComparison.OrdinalIgnoreCase))
                        {
                            bot.set_system_type(side, intValue);
                        }
                        // Special handling for Opponent type - use set_opponent_type method
                        else if (key.Equals("Opponent type", StringComparison.OrdinalIgnoreCase))
                        {
                            bot.set_opponent_type(side, intValue);
                        }
                        else
                        {
                            // For boolean-like values (0/1), use set_conventions with bool
                            bot.set_conventions(side, key, intValue == 1);
                        }
                        success++;
                    }
                    catch (Exception ex)
                    {
                        Console.Error.WriteLine($"[FAILED] Convention '{key}' = {intValue}: {ex.Message}");
                        failed++;
                    }
                }
                else if (valueStr.ToLower() == "true")
                {
                    try { bot.set_conventions(side, key, true); success++; }
                    catch (Exception) { failed++; }
                }
                else if (valueStr.ToLower() == "false")
                {
                    try { bot.set_conventions(side, key, false); success++; }
                    catch (Exception) { failed++; }
                }
            }

            // Log once per run
            if (!_conventionsLogged)
            {
                Console.Error.WriteLine($"Conventions from {Path.GetFileName(filePath)}: {success} success, {failed} failed");
                _conventionsLogged = true;
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

        /// <summary>
        /// Parse vulnerability string to EPBot code.
        /// EPBot encoding (per https://sites.google.com/view/bbaenglish/for-programmers):
        ///   0 = both before (None)
        ///   1 = EW vulnerable
        ///   2 = NS vulnerable
        ///   3 = both vulnerable
        /// </summary>
        static int ParseVulnerability(string vul)
        {
            if (string.IsNullOrEmpty(vul)) return 0;
            switch (vul.ToUpper().Replace("-", ""))
            {
                case "NONE": case "": case "-": return 0;
                case "EW": case "EASTWEST": return 1;
                case "NS": case "NORTHSOUTH": return 2;
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
