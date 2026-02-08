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
        var nsCard = await GetConventionCardFromPbs(scenario) ?? _defaultNsCard;
        return new ConventionCards
        {
            Ns = nsCard,
            Ew = _defaultEwCard
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
    /// Fetch a scenario's .pbs file from GitHub and extract the convention-card property.
    /// </summary>
    private async Task<string?> GetConventionCardFromPbs(string scenario)
    {
        var url = $"{_githubRawBaseUrl}/pbs-release/{scenario}.pbs";

        try
        {
            var response = await _httpClient.GetAsync(url);
            if (!response.IsSuccessStatusCode)
            {
                _logger.LogWarning("PBS file not found on GitHub: {Scenario} (HTTP {StatusCode})", scenario, (int)response.StatusCode);
                return null;
            }

            var content = await response.Content.ReadAsStringAsync();
            var pattern = new Regex(@"^#?\s*convention-card:\s*(.+)$", RegexOptions.IgnoreCase | RegexOptions.Multiline);
            var match = pattern.Match(content);
            if (match.Success)
            {
                var value = match.Groups[1].Value.Trim();
                if (!string.IsNullOrEmpty(value))
                {
                    _logger.LogInformation("Found convention card '{Card}' for scenario '{Scenario}' from GitHub", value, scenario);
                    return value;
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error fetching PBS file from GitHub for scenario: {Scenario}", scenario);
        }

        _logger.LogInformation("No convention card specified for scenario '{Scenario}', using default", scenario);
        return null;
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
