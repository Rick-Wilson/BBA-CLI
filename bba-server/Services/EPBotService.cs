using System.Reflection;
using BbaServer.Models;

namespace BbaServer.Services;

/// <summary>
/// Service for generating auctions using EPBot.
/// Thread-safe with semaphore limiting concurrent EPBot instances.
/// </summary>
public class EPBotService
{
    private readonly ILogger<EPBotService> _logger;
    private readonly ConventionService _conventionService;
    private readonly SemaphoreSlim _semaphore;
    private readonly int _maxConcurrency;

    // Cached EPBot type - loaded once at startup
    private readonly Type? _epbotType;
    private readonly string? _loadedDllName;
    private readonly int _epbotVersion;

    /// <summary>
    /// EPBot internal version number (e.g., 8734).
    /// </summary>
    public int EPBotVersion => _epbotVersion;

    public EPBotService(ILogger<EPBotService> logger, ConventionService conventionService, IConfiguration configuration)
    {
        _logger = logger;
        _conventionService = conventionService;
        _maxConcurrency = configuration.GetValue<int>("EPBot:MaxConcurrency", 4);
        _semaphore = new SemaphoreSlim(_maxConcurrency, _maxConcurrency);

        // Load EPBot assembly once at startup
        (_epbotType, _loadedDllName) = LoadEPBotType();
        if (_epbotType != null)
        {
            _logger.LogInformation("EPBot loaded successfully from {DllName}", _loadedDllName);

            // Get EPBot version
            try
            {
                dynamic bot = CreateEPBot();
                _epbotVersion = bot.version();
                _logger.LogInformation("EPBot version: {Version}", _epbotVersion);
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Could not get EPBot version");
                _epbotVersion = 0;
            }
        }
        else
        {
            _logger.LogError("Failed to load EPBot DLL - auction generation will fail");
            _epbotVersion = 0;
        }
    }

    /// <summary>
    /// Load the EPBot type once at startup.
    /// </summary>
    private (Type? type, string? dllName) LoadEPBotType()
    {
        string baseDir = AppDomain.CurrentDomain.BaseDirectory;
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
                        return (type, dllName);
                    }
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "Failed to load {DllName}", dllName);
                }
            }
        }

        return (null, null);
    }

    /// <summary>
    /// Generate an auction for a deal.
    /// </summary>
    public async Task<AuctionResponse> GenerateAuctionAsync(DealInfo deal, ConventionCards conventions)
    {
        await _semaphore.WaitAsync();
        try
        {
            return await Task.Run(() => GenerateAuction(deal, conventions));
        }
        finally
        {
            _semaphore.Release();
        }
    }

    private AuctionResponse GenerateAuction(DealInfo deal, ConventionCards conventions)
    {
        var response = new AuctionResponse
        {
            ConventionsUsed = conventions
        };

        try
        {
            // Parse the PBN deal
            var (firstSeat, hands) = ParsePbnDeal(deal.Pbn);
            var dealer = ParseDealer(deal.Dealer);
            var vul = ParseVulnerability(deal.Vulnerability);
            var scoring = ParseScoring(deal.Scoring);

            // Get convention file paths
            var nsConvPath = _conventionService.GetConventionFilePath(conventions.Ns);
            var ewConvPath = _conventionService.GetConventionFilePath(conventions.Ew);

            // Verify convention files exist
            if (!File.Exists(nsConvPath))
            {
                response.Error = $"NS convention file not found: {conventions.Ns}";
                return response;
            }
            if (!File.Exists(ewConvPath))
            {
                response.Error = $"EW convention file not found: {conventions.Ew}";
                return response;
            }

            // Create 4 EPBot instances
            dynamic[] players = new dynamic[4];
            string[] posNames = { "N", "E", "S", "W" };

            for (int i = 0; i < 4; i++)
            {
                players[i] = CreateEPBot();
                string[] hand = hands[i];
                players[i].new_hand(i, ref hand, dealer, vul, false, false);
                players[i].scoring = scoring; // 0 = MP, 1 = IMP

                // Load conventions
                LoadConventions(players[i], nsConvPath, 0);
                LoadConventions(players[i], ewConvPath, 1);

                // Debug: verify key conventions loaded
                if (i == 0)
                {
                    try
                    {
                        bool smolenNS = players[i].get_conventions(0, "SMOLEN");
                        bool smolenEW = players[i].get_conventions(1, "SMOLEN");
                        bool garbageStaymanNS = players[i].get_conventions(0, "Garbage Stayman");
                        _logger.LogInformation("Conventions check: SMOLEN NS={SmolenNS}, EW={SmolenEW}, GarbageStayman={GS}",
                            smolenNS, smolenEW, garbageStaymanNS);
                    }
                    catch (Exception ex)
                    {
                        _logger.LogWarning("Could not verify conventions: {Error}", ex.Message);
                    }
                }
            }

            // Generate auction
            var bids = new List<string>();
            var meanings = new List<BidMeaning>();
            int currentPos = dealer;
            int passCount = 0;
            bool hasBid = false;

            for (int round = 0; round < 100; round++)
            {
                int bidCode = players[currentPos].get_bid();
                string bidStr = DecodeBid(bidCode);
                bids.Add(bidStr);

                // Broadcast bid to all players FIRST
                // Partner needs to receive the bid before they can explain what it means
                for (int j = 0; j < 4; j++)
                {
                    players[j].set_bid(currentPos, bidCode);
                }

                // NOW get bid meaning from PARTNER's perspective (after they've seen the bid)
                int partnerPos = (currentPos + 2) % 4;
                string? meaning = null;
                bool isAlert = false;
                try
                {
                    isAlert = players[partnerPos].get_info_alerting(currentPos);
                    if (isAlert)
                    {
                        meaning = players[partnerPos].get_info_meaning(currentPos);
                    }
                }
                catch { }

                meanings.Add(new BidMeaning
                {
                    Position = bids.Count - 1,
                    Bid = bidStr,
                    Meaning = meaning,
                    IsAlert = isAlert
                });

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

            response.Success = true;
            response.Auction = bids;
            response.AuctionEncoded = EncodeToBboFormat(bids);
            response.Meanings = meanings;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error generating auction");
            response.Error = ex.Message;
        }

        return response;
    }

    /// <summary>
    /// Create an EPBot instance from the cached type.
    /// </summary>
    private dynamic CreateEPBot()
    {
        if (_epbotType == null)
        {
            throw new Exception("EPBot DLL not loaded");
        }

        return Activator.CreateInstance(_epbotType)!;
    }

    /// <summary>
    /// Load conventions from a .bbsa file.
    /// </summary>
    private void LoadConventions(dynamic bot, string filePath, int side)
    {
        foreach (var line in File.ReadAllLines(filePath))
        {
            if (string.IsNullOrWhiteSpace(line) || line.StartsWith("#") || line.StartsWith(";"))
                continue;

            var parts = line.Split(new[] { '=' }, 2);
            if (parts.Length != 2) continue;

            string key = parts[0].Trim();
            string valueStr = parts[1].Trim();

            try
            {
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
                    else
                    {
                        bot.set_conventions(side, key, intValue == 1);
                    }
                }
                else if (valueStr.Equals("true", StringComparison.OrdinalIgnoreCase))
                {
                    bot.set_conventions(side, key, true);
                }
                else if (valueStr.Equals("false", StringComparison.OrdinalIgnoreCase))
                {
                    bot.set_conventions(side, key, false);
                }
            }
            catch { /* Ignore unknown conventions */ }
        }
    }

    /// <summary>
    /// Parse PBN deal format.
    /// </summary>
    private (int firstSeat, string[][] hands) ParsePbnDeal(string pbn)
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

            // PBN format is S.H.D.C but EPBot expects C.D.H.S
            Array.Reverse(suits);
            hands[pos] = suits;
        }

        return (firstSeat, hands);
    }

    private int ParseDealer(string dealer)
    {
        return dealer.ToUpper() switch
        {
            "N" or "NORTH" => 0,
            "E" or "EAST" => 1,
            "S" or "SOUTH" => 2,
            "W" or "WEST" => 3,
            _ => 0
        };
    }

    private int ParseVulnerability(string vul)
    {
        return vul.ToUpper().Replace("-", "") switch
        {
            "NONE" or "" => 0,
            "EW" or "EASTWEST" => 1,
            "NS" or "NORTHSOUTH" => 2,
            "BOTH" or "ALL" => 3,
            _ => 0
        };
    }

    private int ParseScoring(string scoring)
    {
        return scoring.ToUpper() switch
        {
            "IMP" or "IMPS" => 1,
            "MP" or "MATCHPOINTS" or "" => 0,
            _ => 0
        };
    }

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

    /// <summary>
    /// Encode auction to BBO 2-char format.
    /// </summary>
    private string EncodeToBboFormat(List<string> bids)
    {
        var result = new System.Text.StringBuilder();
        foreach (var bid in bids)
        {
            string encoded = bid switch
            {
                "Pass" => "--",
                "X" => "Db",
                "XX" => "Rd",
                _ when bid.EndsWith("NT") => bid[0] + "N",
                _ => bid.Length == 2 ? bid : bid
            };
            result.Append(encoded.PadRight(2).Substring(0, 2));
        }
        return result.ToString();
    }
}
