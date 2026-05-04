using System.Text.RegularExpressions;
using BbaServer.Models;

namespace BbaServer.Services;

/// <summary>
/// Service for looking up convention cards from GitHub.
/// Fetches .bbsa files from raw.githubusercontent.com and .pbs files for scenario convention lookups.
/// </summary>
public class ConventionService
{
    private readonly ILogger<ConventionService> _logger;
    private readonly HttpClient _httpClient = new();
    private readonly string _defaultNsCard;
    private readonly string _defaultEwCard;
    private readonly string _githubRawBaseUrl;

    public ConventionService(ILogger<ConventionService> logger, IConfiguration configuration)
    {
        _logger = logger;

        _defaultNsCard = configuration["Pbs:DefaultNsCard"] ?? "21GF-DEFAULT";
        _defaultEwCard = configuration["Pbs:DefaultEwCard"] ?? "21GF-GIB";
        _githubRawBaseUrl = configuration["GitHub:RawBaseUrl"]
            ?? "https://raw.githubusercontent.com/ADavidBailey/Practice-Bidding-Scenarios/main";
    }

    /// <summary>
    /// Get convention cards for a scenario by fetching its .pbs file from GitHub.
    /// </summary>
    public async Task<ConventionCards> GetConventionsForScenarioAsync(string scenario)
    {
        var (nsCard, ewCard) = await GetConventionCardsFromPbs(scenario);
        return new ConventionCards
        {
            Ns = nsCard ?? _defaultNsCard,
            Ew = ewCard ?? _defaultEwCard
        };
    }

    /// <summary>
    /// Fetch a .bbsa convention card file from GitHub and return its lines.
    /// </summary>
    public async Task<string[]> GetBbsaContentAsync(string cardName)
    {
        var url = $"{_githubRawBaseUrl}/bbsa/{cardName}.bbsa";
        _logger.LogInformation("Fetching convention card from GitHub: {Url}", url);

        var response = await _httpClient.GetAsync(url);
        if (!response.IsSuccessStatusCode)
        {
            throw new Exception($"Convention card not found on GitHub: {cardName} (HTTP {(int)response.StatusCode})");
        }

        var content = await response.Content.ReadAsStringAsync();
        return content.Split('\n');
    }

    /// <summary>
    /// Fetch a scenario's .pbs file from GitHub and extract convention card properties.
    /// Supports both new format (convention-card-ns/ew) and old format (convention-card).
    /// </summary>
    private async Task<(string? ns, string? ew)> GetConventionCardsFromPbs(string scenario)
    {
        var url = $"{_githubRawBaseUrl}/pbs-release/{scenario}.pbs";

        try
        {
            var response = await _httpClient.GetAsync(url);
            if (!response.IsSuccessStatusCode)
            {
                _logger.LogWarning("PBS file not found on GitHub: {Scenario} (HTTP {StatusCode})", scenario, (int)response.StatusCode);
                return (null, null);
            }

            var content = await response.Content.ReadAsStringAsync();

            // Try new format first: convention-card-ns and convention-card-ew
            var nsPattern = new Regex(@"^convention-card-ns:\s*(.+)$", RegexOptions.IgnoreCase | RegexOptions.Multiline);
            var ewPattern = new Regex(@"^convention-card-ew:\s*(.+)$", RegexOptions.IgnoreCase | RegexOptions.Multiline);
            var nsMatch = nsPattern.Match(content);
            var ewMatch = ewPattern.Match(content);

            if (nsMatch.Success || ewMatch.Success)
            {
                var ns = nsMatch.Success ? nsMatch.Groups[1].Value.Trim() : null;
                var ew = ewMatch.Success ? ewMatch.Groups[1].Value.Trim() : null;
                _logger.LogInformation("Found convention cards NS='{Ns}', EW='{Ew}' for scenario '{Scenario}'",
                    ns ?? "(default)", ew ?? "(default)", scenario);
                return (ns, ew);
            }

            // Fall back to old format: convention-card (NS only)
            var oldPattern = new Regex(@"^#?\s*convention-card:\s*(.+)$", RegexOptions.IgnoreCase | RegexOptions.Multiline);
            var oldMatch = oldPattern.Match(content);
            if (oldMatch.Success)
            {
                var value = oldMatch.Groups[1].Value.Trim();
                if (!string.IsNullOrEmpty(value))
                {
                    _logger.LogInformation("Found convention card '{Card}' (old format) for scenario '{Scenario}'", value, scenario);
                    return (value, null);
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error fetching PBS file from GitHub for scenario: {Scenario}", scenario);
        }

        _logger.LogInformation("No convention card specified for scenario '{Scenario}', using defaults", scenario);
        return (null, null);
    }

    /// <summary>
    /// Get the list of available scenarios from GitHub's pbs-release folder.
    /// </summary>
    public async Task<List<string>> GetScenarioListAsync()
    {
        var url = "https://api.github.com/repos/ADavidBailey/Practice-Bidding-Scenarios/contents/pbs-release";
        _logger.LogInformation("Fetching scenario list from GitHub API");

        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.Add("User-Agent", "BBA-Server");

        var response = await _httpClient.SendAsync(request);
        if (!response.IsSuccessStatusCode)
        {
            _logger.LogWarning("Failed to fetch scenario list from GitHub (HTTP {StatusCode})", (int)response.StatusCode);
            return new List<string>();
        }

        var json = await response.Content.ReadAsStringAsync();
        var items = System.Text.Json.JsonSerializer.Deserialize<List<GitHubContentItem>>(json);
        if (items == null) return new List<string>();

        return items
            .Where(i => i.name?.EndsWith(".pbs") == true)
            .Select(i => Path.GetFileNameWithoutExtension(i.name!))
            .OrderBy(s => s)
            .ToList();
    }

    private class GitHubContentItem
    {
        public string? name { get; set; }
        public string? type { get; set; }
    }
}
