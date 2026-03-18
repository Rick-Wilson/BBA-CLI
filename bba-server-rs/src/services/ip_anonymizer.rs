use sha2::{Digest, Sha256};

const SALT: &str = "BBA-Server-2024-Salt";

const FIRST_NAMES: &[&str] = &[
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
    "Wayne", "Wendy", "William", "Zachary",
];

const SURNAMES: &[&str] = &[
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
    "Watson", "West", "White", "Williams", "Wilson", "Wood", "Wright", "Young",
];

/// Anonymize an IP address into a friendly name like "Alice_Baker".
/// Deterministic: same IP always produces the same name.
pub fn anonymize(ip: Option<&str>) -> String {
    let ip = match ip {
        Some(ip) if !ip.is_empty() => ip,
        _ => return "Unknown_Visitor".to_string(),
    };

    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}", SALT, ip).as_bytes());
    let hash = hasher.finalize();

    let hash_num = u64::from_le_bytes(hash[..8].try_into().unwrap());
    let first_idx = (hash_num % FIRST_NAMES.len() as u64) as usize;
    let surname_idx = ((hash_num / FIRST_NAMES.len() as u64) % SURNAMES.len() as u64) as usize;

    format!("{}_{}", FIRST_NAMES[first_idx], SURNAMES[surname_idx])
}
