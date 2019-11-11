use std::cmp;
use std::convert::TryInto;
use std::io;
use std::io::Write;
use std::iter;
use std::str;
use std::time;

//   ____                _              _
//  / ___|___  _ __  ___| |_ __ _ _ __ | |_ ___
// | |   / _ \| '_ \/ __| __/ _` | '_ \| __/ __|
// | |__| (_) | | | \__ \ || (_| | | | | |_\__ \
//  \____\___/|_| |_|___/\__\__,_|_| |_|\__|___/
//

static BAR_SIZE: usize = 40;
static REFRESH_DELAY: u128 = 100;

static PREFIXES: &[&str] = &["it", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi", "Yi"];
static SPINNER: &str =
    "⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ ";

//  ____
// |  _ \ _ __ ___   __ _ _ __ ___  ___ ___
// | |_) | '__/ _ \ / _` | '__/ _ \/ __/ __|
// |  __/| | | (_) | (_| | | |  __/\__ \__ \
// |_|   |_|  \___/ \__, |_|  \___||___/___/
//                  |___/

pub struct Progress<T, U>
where
    T: Iterator<Item = U>,
{
    /// Iterator that it actualy extracts from
    iterator: T,
    /// Hypotetical size of the iterator
    max_iterations: usize,
    /// Number of elements already extracted
    count_iterations: usize,

    /// Creation instant of the progress bar
    start_time: time::Instant,

    /// Wether the bar should automaticaly refresh while iterating it
    auto_refresh: bool,

    /// Last refresh instant
    last_refresh: time::Instant,
    /// Width of the bar during the previous refresh
    last_width: usize,

    /// Extra infos to display during loading
    extra_msg: Option<String>,

    /// Purely estetic looping animation
    spinner: iter::Cycle<str::Chars<'static>>,
}

impl<T, U> Progress<T, U>
where
    T: Iterator<Item = U>,
{
    pub fn auto_refresh(mut self, toggle: bool) -> Progress<T, U> {
        self.auto_refresh = toggle;
        self
    }

    pub fn extra_msg(&mut self, msg: String) {
        self.extra_msg = Some(msg);
    }

    pub fn refresh(&mut self) {
        // Compute bar shape
        let proportion = self.count_iterations as f64 / self.max_iterations as f64;
        let body_length = cmp::min(
            BAR_SIZE + 1,
            (proportion * (BAR_SIZE + 1) as f64).round() as usize,
        );
        let mut void_length = (BAR_SIZE + 1) - body_length;
        let mut has_head = false;

        if void_length > 0 {
            void_length -= 1;
            has_head = true;
        }

        let body = "=".repeat(body_length);
        let void = " ".repeat(void_length);
        let head = ">".repeat(has_head.into());

        // Compute speed
        let mut speed = 1_000_000. * self.count_iterations as f64
            / self.start_time.elapsed().as_micros() as f64;
        let mut prefix_index = 0;

        while speed > 1_024. && prefix_index + 1 < PREFIXES.len() {
            speed /= 1_024.;
            prefix_index += 1;
        }

        // Display
        let elapsed = self.start_time.elapsed().as_secs();

        let mut display = format!(
            "{} [{}{}{}]  {:02}:{:02}  {:.2} {}/s",
            self.spinner.next().unwrap(),
            body,
            head,
            void,
            elapsed / 60,
            elapsed % 60,
            speed,
            PREFIXES[prefix_index],
        );

        if let Some(msg) = &self.extra_msg {
            display = format!("{} -- {}", display, msg);
        }

        eprint!("\r{}", display);

        if display.chars().count() < self.last_width {
            eprint!("{}", " ".repeat(self.last_width - display.chars().count()))
        }

        io::stdout().flush().expect("Can't flush stdout");

        // Update informations about last refresh
        self.last_refresh = time::Instant::now();
        self.last_width = display.chars().count();
    }
}

impl<T, U> Progress<T, U>
where
    T: std::iter::Iterator<Item = U>,
{
    pub fn from_iter(iterator: T) -> Progress<T, U> {
        let max_iterations = iterator
            .size_hint().1
            .expect("Impossible to init progress bar for objects larger than a 64 bits integer");

        Progress {
            iterator,
            max_iterations,
            count_iterations: 0,
            start_time: time::Instant::now(),
            auto_refresh: true,
            last_refresh: time::Instant::now(),
            last_width: 0,
            extra_msg: None,
            spinner: SPINNER.chars().cycle(),
        }
    }
}

impl<T, U> Iterator for Progress<T, U>
where
    T: Iterator<Item = U>,
{
    type Item = U;

    fn next(&mut self) -> Option<U> {
        let ret = self.iterator.next();

        if self.auto_refresh {
            match ret {
                None => {
                    self.refresh();
                    print!("\n");
                }
                Some(_) => {
                    if self.last_refresh.elapsed().as_millis() > REFRESH_DELAY {
                        self.refresh();
                    }
                }
            }
        }

        self.count_iterations += 1;
        ret
    }
}
