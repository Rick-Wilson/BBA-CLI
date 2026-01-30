namespace BbaServer.Services;

/// <summary>
/// Service for admin dashboard operations - log viewing and statistics.
/// </summary>
public class AdminService
{
    private readonly string _logDirectory;
    private readonly HashSet<string> _allowedIPs;

    // Raw IPs that are considered "localhost" and always allowed
    private static readonly HashSet<string> _localhostIPs = new(StringComparer.OrdinalIgnoreCase)
    {
        "127.0.0.1",
        "::1",
        "localhost"
    };

    public AdminService(IConfiguration configuration)
    {
        _logDirectory = Path.Combine(AppContext.BaseDirectory, "logs");

        // Whitelist of anonymized IPs allowed to access admin
        _allowedIPs = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "Valerie_Perez",  // David (external)
            "Travis_Scott",   // Rick (external)
            "Tom_Martinez"    // Rick (local via Parallels 10.211.55.2)
        };
    }

    /// <summary>
    /// Check if access is allowed based on raw IP or anonymized IP.
    /// </summary>
    public bool IsAllowed(string? rawIP, string? anonIP)
    {
        // Always allow localhost
        if (!string.IsNullOrEmpty(rawIP) && _localhostIPs.Contains(rawIP))
            return true;

        // Check anonymized IP whitelist
        if (!string.IsNullOrEmpty(anonIP) && _allowedIPs.Contains(anonIP))
            return true;

        return false;
    }

    /// <summary>
    /// Get debug info about the current connection.
    /// </summary>
    public object GetConnectionInfo(string? rawIP, string? anonIP)
    {
        return new
        {
            rawIP = rawIP ?? "unknown",
            anonymizedIP = anonIP ?? "unknown",
            isAllowed = IsAllowed(rawIP, anonIP),
            whitelistedNames = _allowedIPs.ToList()
        };
    }

    /// <summary>
    /// Get list of available log files.
    /// </summary>
    public List<LogFileInfo> GetLogFiles()
    {
        var files = new List<LogFileInfo>();

        if (!Directory.Exists(_logDirectory))
            return files;

        foreach (var file in Directory.GetFiles(_logDirectory, "*.*")
            .OrderByDescending(f => File.GetLastWriteTime(f)))
        {
            var info = new FileInfo(file);
            files.Add(new LogFileInfo
            {
                Name = info.Name,
                Size = info.Length,
                LastModified = info.LastWriteTime,
                Type = GetFileType(info.Name)
            });
        }

        return files;
    }

    /// <summary>
    /// Get content of a specific log file.
    /// </summary>
    public string? GetLogContent(string filename)
    {
        // Security: only allow files from log directory, no path traversal
        var safeName = Path.GetFileName(filename);
        var filePath = Path.Combine(_logDirectory, safeName);

        if (!File.Exists(filePath))
            return null;

        return File.ReadAllText(filePath);
    }

    /// <summary>
    /// Get parsed CSV data for auction logs.
    /// </summary>
    public List<Dictionary<string, string>> GetAuctionLogData(string? filename = null)
    {
        var targetFile = filename ?? GetLatestAuctionLog();
        if (targetFile == null) return new List<Dictionary<string, string>>();

        var safeName = Path.GetFileName(targetFile);
        var filePath = Path.Combine(_logDirectory, safeName);

        if (!File.Exists(filePath))
            return new List<Dictionary<string, string>>();

        return ParseCsv(filePath);
    }

    /// <summary>
    /// Get summary statistics from auction logs.
    /// </summary>
    public AuditStats GetStats()
    {
        var stats = new AuditStats();
        var latestLog = GetLatestAuctionLog();

        if (latestLog == null)
            return stats;

        var data = GetAuctionLogData(latestLog);

        stats.TotalRequests = data.Count;
        stats.SuccessfulRequests = data.Count(d => d.GetValueOrDefault("Success") == "true");
        stats.FailedRequests = stats.TotalRequests - stats.SuccessfulRequests;

        // Requests by scenario
        stats.RequestsByScenario = data
            .Where(d => !string.IsNullOrEmpty(d.GetValueOrDefault("Scenario")))
            .GroupBy(d => d.GetValueOrDefault("Scenario") ?? "")
            .ToDictionary(g => g.Key, g => g.Count());

        // Requests by user (anonymized IP)
        stats.RequestsByUser = data
            .GroupBy(d => d.GetValueOrDefault("RequestIP") ?? "unknown")
            .ToDictionary(g => g.Key, g => g.Count());

        // Requests by day
        stats.RequestsByDay = data
            .Where(d => d.ContainsKey("Timestamp"))
            .GroupBy(d => d["Timestamp"].Split(' ')[0])
            .ToDictionary(g => g.Key, g => g.Count());

        // Average duration
        var durations = data
            .Where(d => int.TryParse(d.GetValueOrDefault("DurationMs"), out _))
            .Select(d => int.Parse(d["DurationMs"]))
            .ToList();

        if (durations.Any())
        {
            stats.AverageDurationMs = (int)durations.Average();
            stats.MaxDurationMs = durations.Max();
        }

        // Recent errors
        stats.RecentErrors = data
            .Where(d => d.GetValueOrDefault("Success") == "false" && !string.IsNullOrEmpty(d.GetValueOrDefault("Error")))
            .TakeLast(10)
            .Select(d => new ErrorInfo
            {
                Timestamp = d.GetValueOrDefault("Timestamp") ?? "",
                Error = d.GetValueOrDefault("Error") ?? "",
                Scenario = d.GetValueOrDefault("Scenario") ?? ""
            })
            .ToList();

        return stats;
    }

    private string? GetLatestAuctionLog()
    {
        if (!Directory.Exists(_logDirectory))
            return null;

        return Directory.GetFiles(_logDirectory, "audit-auction-*.csv")
            .OrderByDescending(f => f)
            .FirstOrDefault();
    }

    private static string GetFileType(string filename)
    {
        if (filename.StartsWith("audit-auction")) return "auction-csv";
        if (filename.StartsWith("audit-scenario")) return "scenario-csv";
        if (filename.EndsWith(".log")) return "log";
        if (filename.EndsWith(".csv")) return "csv";
        return "other";
    }

    private static List<Dictionary<string, string>> ParseCsv(string filePath)
    {
        var result = new List<Dictionary<string, string>>();
        var lines = File.ReadAllLines(filePath);

        if (lines.Length < 2) return result;

        var headers = ParseCsvLine(lines[0]);

        for (int i = 1; i < lines.Length; i++)
        {
            var values = ParseCsvLine(lines[i]);
            var row = new Dictionary<string, string>();

            for (int j = 0; j < headers.Count && j < values.Count; j++)
            {
                row[headers[j]] = values[j];
            }

            result.Add(row);
        }

        return result;
    }

    private static List<string> ParseCsvLine(string line)
    {
        var result = new List<string>();
        var current = "";
        var inQuotes = false;

        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];

            if (c == '"')
            {
                if (inQuotes && i + 1 < line.Length && line[i + 1] == '"')
                {
                    current += '"';
                    i++;
                }
                else
                {
                    inQuotes = !inQuotes;
                }
            }
            else if (c == ',' && !inQuotes)
            {
                result.Add(current);
                current = "";
            }
            else
            {
                current += c;
            }
        }

        result.Add(current);
        return result;
    }
}

public class LogFileInfo
{
    public string Name { get; set; } = "";
    public long Size { get; set; }
    public DateTime LastModified { get; set; }
    public string Type { get; set; } = "";
}

public class AuditStats
{
    public int TotalRequests { get; set; }
    public int SuccessfulRequests { get; set; }
    public int FailedRequests { get; set; }
    public int AverageDurationMs { get; set; }
    public int MaxDurationMs { get; set; }
    public Dictionary<string, int> RequestsByScenario { get; set; } = new();
    public Dictionary<string, int> RequestsByUser { get; set; } = new();
    public Dictionary<string, int> RequestsByDay { get; set; } = new();
    public List<ErrorInfo> RecentErrors { get; set; } = new();
}

public class ErrorInfo
{
    public string Timestamp { get; set; } = "";
    public string Error { get; set; } = "";
    public string Scenario { get; set; } = "";
}
