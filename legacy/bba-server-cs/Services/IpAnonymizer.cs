using System.Security.Cryptography;
using System.Text;

namespace BbaServer.Services;

/// <summary>
/// Anonymizes IP addresses into friendly names like "Alice_Baker"
/// </summary>
public static class IpAnonymizer
{
    private static readonly string[] FirstNames = {
        "Aaron", "Abigail", "Adam", "Adrian", "Aiden", "Alex", "Alice", "Allison",
        "Amanda", "Amber", "Amy", "Andrea", "Andrew", "Angela", "Anna", "Anthony",
        "Ashley", "Austin", "Barbara", "Benjamin", "Beth", "Brandon", "Brenda",
        "Brian", "Brittany", "Bruce", "Bryan", "Caleb", "Cameron", "Carl", "Carlos",
        "Carol", "Caroline", "Catherine", "Charles", "Charlotte", "Chelsea", "Chris",
        "Christina", "Christine", "Christopher", "Cindy", "Claire", "Clara", "Cody",
        "Colin", "Connor", "Craig", "Crystal", "Cynthia", "Dale", "Daniel", "Danielle",
        "David", "Dawn", "Deborah", "Dennis", "Derek", "Diana", "Diane", "Donald",
        "Donna", "Dorothy", "Douglas", "Dylan", "Edward", "Eileen", "Eleanor", "Elizabeth",
        "Ellen", "Emily", "Emma", "Eric", "Erica", "Erin", "Ethan", "Eugene", "Eva",
        "Evan", "Evelyn", "Frances", "Francis", "Frank", "Gabriel", "Gary", "George",
        "Gerald", "Gloria", "Grace", "Gregory", "Hannah", "Harold", "Harry", "Heather",
        "Helen", "Henry", "Holly", "Howard", "Ian", "Isaac", "Isabella", "Jack", "Jacob",
        "James", "Jamie", "Jane", "Janet", "Jason", "Jean", "Jeffrey", "Jennifer",
        "Jeremy", "Jerry", "Jesse", "Jessica", "Jill", "Joan", "Joe", "Joel", "John",
        "Jonathan", "Jordan", "Joseph", "Joshua", "Joyce", "Julia", "Julie", "Justin",
        "Karen", "Katherine", "Kathleen", "Katie", "Keith", "Kelly", "Kenneth", "Kevin",
        "Kim", "Kimberly", "Kyle", "Larry", "Laura", "Lauren", "Lawrence", "Leah",
        "Leonard", "Leslie", "Linda", "Lisa", "Logan", "Louis", "Lucas", "Lucy", "Luke",
        "Madison", "Margaret", "Maria", "Marie", "Mark", "Martha", "Martin", "Mary",
        "Mason", "Matthew", "Megan", "Melanie", "Melissa", "Michael", "Michelle", "Mike",
        "Monica", "Nancy", "Natalie", "Nathan", "Nicholas", "Nicole", "Noah", "Oliver",
        "Olivia", "Oscar", "Pamela", "Patricia", "Patrick", "Paul", "Paula", "Peter",
        "Philip", "Rachel", "Ralph", "Randy", "Raymond", "Rebecca", "Richard", "Robert",
        "Robin", "Roger", "Ronald", "Rose", "Roy", "Russell", "Ruth", "Ryan", "Samantha",
        "Samuel", "Sandra", "Sara", "Sarah", "Scott", "Sean", "Sharon", "Sophia",
        "Stephanie", "Stephen", "Steve", "Steven", "Susan", "Tammy", "Teresa", "Terry",
        "Thomas", "Tiffany", "Timothy", "Todd", "Tom", "Tony", "Tracy", "Travis", "Tyler",
        "Valerie", "Vanessa", "Victor", "Victoria", "Vincent", "Virginia", "Walter",
        "Wayne", "Wendy", "William", "Zachary"
    };

    private static readonly string[] Surnames = {
        "Adams", "Allen", "Anderson", "Bailey", "Baker", "Barnes", "Bell", "Bennett",
        "Brooks", "Brown", "Bryant", "Butler", "Campbell", "Carter", "Clark", "Coleman",
        "Collins", "Cook", "Cooper", "Cox", "Cruz", "Davis", "Diaz", "Edwards", "Evans",
        "Fisher", "Flores", "Ford", "Foster", "Garcia", "Gibson", "Gomez", "Gonzalez",
        "Gordon", "Graham", "Gray", "Green", "Griffin", "Hall", "Hamilton", "Harris",
        "Harrison", "Hayes", "Henderson", "Hernandez", "Hill", "Holmes", "Howard",
        "Hughes", "Hunt", "Jackson", "James", "Jenkins", "Johnson", "Jones", "Jordan",
        "Kelly", "Kennedy", "Kim", "King", "Lee", "Lewis", "Long", "Lopez", "Marshall",
        "Martin", "Martinez", "Mason", "Matthews", "Miller", "Mitchell", "Moore",
        "Morales", "Morgan", "Morris", "Murphy", "Murray", "Nelson", "Nguyen", "Ortiz",
        "Owens", "Parker", "Patterson", "Perez", "Perry", "Peterson", "Phillips",
        "Powell", "Price", "Ramirez", "Reed", "Reyes", "Reynolds", "Richardson", "Rivera",
        "Roberts", "Robinson", "Rodriguez", "Rogers", "Ross", "Russell", "Sanchez",
        "Sanders", "Scott", "Simmons", "Smith", "Stewart", "Sullivan", "Taylor", "Thomas",
        "Thompson", "Torres", "Turner", "Walker", "Wallace", "Ward", "Washington",
        "Watson", "West", "White", "Williams", "Wilson", "Wood", "Wright", "Young"
    };

    private const string Salt = "BBA-Server-2024-Salt";

    /// <summary>
    /// Anonymize an IP address into a friendly name like "Alice_Baker"
    /// </summary>
    public static string Anonymize(string? ip)
    {
        if (string.IsNullOrEmpty(ip)) return "Unknown_Visitor";

        using var sha256 = SHA256.Create();
        var bytes = Encoding.UTF8.GetBytes(Salt + ip);
        var hash = sha256.ComputeHash(bytes);

        // Convert first 8 bytes of hash to a number for indexing
        var hashNum = BitConverter.ToUInt64(hash, 0);

        // Pick first name and surname from the hash
        var firstIdx = (int)(hashNum % (ulong)FirstNames.Length);
        var surnameIdx = (int)((hashNum / (ulong)FirstNames.Length) % (ulong)Surnames.Length);

        return $"{FirstNames[firstIdx]}_{Surnames[surnameIdx]}";
    }
}
