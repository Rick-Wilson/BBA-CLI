namespace BbaServer.Services;

/// <summary>
/// CSV audit logger for tracking auction generation requests.
/// Writes to monthly CSV files with 30-day retention.
/// </summary>
public class AuditLogService
{
    private readonly string _logDirectory;
    private readonly int _retentionDays;
    private readonly string _version;
    private static readonly object _lock = new();

    // CSV headers
    private const string AuctionCsvHeader = "Timestamp,RequestIP,ClientVersion,DurationMs,Version,EPBotVersion,Dealer,Vulnerability,Scoring,NSConvention,EWConvention,Scenario,PBN,Success,Auction,Alerts,Error";
    private const string ScenarioCsvHeader = "Timestamp,RequestIP,ClientVersion,Version,Scenario";

    public AuditLogService(IConfiguration configuration)
    {
        _logDirectory = Path.Combine(AppContext.BaseDirectory, "logs");
        _retentionDays = 30;
        _version = configuration["Version"] ?? "1.0.0";

        Directory.CreateDirectory(_logDirectory);
        CleanupOldLogs();
    }

    /// <summary>
    /// Log an auction generation request.
    /// </summary>
    public void LogRequest(
        string? requestIP,
        string? clientVersion,
        long durationMs,
        int epbotVersion,
        string dealer,
        string vulnerability,
        string scoring,
        string nsConvention,
        string ewConvention,
        string? scenario,
        string pbn,
        bool success,
        string? auction,
        string? alerts,
        string? error)
    {
        var logFile = Path.Combine(_logDirectory, $"audit-auction-{DateTime.Now:yyyy-MM}.csv");
        var timestamp = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss");

        // Format auction for readability
        var formattedAuction = FormatAuction(auction);

        // Escape CSV fields
        var row = string.Join(",",
            EscapeCsv(timestamp),
            EscapeCsv(requestIP ?? "unknown"),
            EscapeCsv(clientVersion ?? ""),
            durationMs.ToString(),
            EscapeCsv(_version),
            epbotVersion.ToString(),
            EscapeCsv(dealer),
            EscapeCsv(vulnerability),
            EscapeCsv(scoring),
            EscapeCsv(nsConvention),
            EscapeCsv(ewConvention),
            EscapeCsv(scenario ?? ""),
            EscapeCsv(pbn),
            success ? "true" : "false",
            EscapeCsv(formattedAuction),
            EscapeCsv(alerts ?? ""),
            EscapeCsv(error ?? "")
        );

        lock (_lock)
        {
            try
            {
                // Write header if file doesn't exist
                if (!File.Exists(logFile))
                {
                    File.WriteAllText(logFile, AuctionCsvHeader + Environment.NewLine);
                }

                File.AppendAllText(logFile, row + Environment.NewLine);
            }
            catch
            {
                // Ignore write errors
            }
        }
    }

    /// <summary>
    /// Log a scenario selection event.
    /// </summary>
    public void LogScenarioSelection(
        string? requestIP,
        string? clientVersion,
        string scenario)
    {
        var logFile = Path.Combine(_logDirectory, $"audit-scenario-{DateTime.Now:yyyy-MM}.csv");
        var timestamp = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss");

        var row = string.Join(",",
            EscapeCsv(timestamp),
            EscapeCsv(requestIP ?? "unknown"),
            EscapeCsv(clientVersion ?? ""),
            EscapeCsv(_version),
            EscapeCsv(scenario)
        );

        lock (_lock)
        {
            try
            {
                // Write header if file doesn't exist
                if (!File.Exists(logFile))
                {
                    File.WriteAllText(logFile, ScenarioCsvHeader + Environment.NewLine);
                }

                File.AppendAllText(logFile, row + Environment.NewLine);
            }
            catch
            {
                // Ignore write errors
            }
        }
    }

    /// <summary>
    /// Format BBO-encoded auction to readable format.
    /// Converts 2-char codes to readable bids, replaces trailing passes with AllPass/PassOut.
    /// </summary>
    private static string FormatAuction(string? encoded)
    {
        if (string.IsNullOrEmpty(encoded))
            return "";

        var bids = new List<string>();

        // Parse 2-char encoded bids
        for (int i = 0; i < encoded.Length; i += 2)
        {
            if (i + 1 >= encoded.Length) break;

            string code = encoded.Substring(i, 2);
            string bid = code switch
            {
                "--" => "Pass",
                "Db" => "X",
                "Rd" => "XX",
                _ when code[1] == 'N' => $"{code[0]}NT",
                _ => code.Trim()
            };
            bids.Add(bid);
        }

        // Replace 4 passes with PassOut
        if (bids.Count == 4 && bids.All(b => b == "Pass"))
        {
            return "PassOut";
        }

        // Replace trailing 3 passes with AllPass
        if (bids.Count >= 4 && bids.TakeLast(3).All(b => b == "Pass"))
        {
            var activeBids = bids.Take(bids.Count - 3).ToList();
            activeBids.Add("AllPass");
            return string.Join(" ", activeBids);
        }

        return string.Join(" ", bids);
    }

    private static string EscapeCsv(string value)
    {
        if (string.IsNullOrEmpty(value))
            return "";

        // If value contains comma, quote, or newline, wrap in quotes and escape quotes
        if (value.Contains(',') || value.Contains('"') || value.Contains('\n') || value.Contains('\r'))
        {
            return "\"" + value.Replace("\"", "\"\"") + "\"";
        }

        return value;
    }

    private void CleanupOldLogs()
    {
        try
        {
            var cutoff = DateTime.Now.AddDays(-_retentionDays);
            foreach (var file in Directory.GetFiles(_logDirectory, "audit-*.csv"))
            {
                if (File.GetLastWriteTime(file) < cutoff)
                {
                    File.Delete(file);
                }
            }
        }
        catch
        {
            // Ignore cleanup errors
        }
    }
}
