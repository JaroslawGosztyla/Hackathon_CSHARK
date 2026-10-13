// Break tips and good working practices (research notes in the docs; sources per tip).
// Data only plus a deterministic pick: the main window's break card and the phase popup must show the same tip.
// Wording rule (CLAUDE.md, rule 10): suggestions only, never health promises. `en` is kept for the translation pass.

export type TipCategory = "eyes" | "stretch" | "move" | "hydrate" | "breathe" | "posture" | "outside" | "social" | "mind" | "tidy";

/** Break length a tip fits: `short` (< 10 min), `long` (10 min or more) or `any`. */
export type TipLength = "short" | "long" | "any";

export interface BreakTip {
  id: string;
  category: TipCategory;
  length: TipLength;
  pl: string;
  en: string;
  /** Where the suggestion comes from (opened in the browser); null = common practice without one source. */
  source: string | null;
}

export interface WorkingTip {
  id: string;
  pl: string;
  en: string;
  source: string;
}

/** Icon per category (decorative; the category name is shown as text next to it). */
export const CATEGORY_ICON: Record<TipCategory, string> = {
  eyes: "👀",
  stretch: "🙆",
  move: "🚶",
  hydrate: "💧",
  breathe: "🌬️",
  posture: "🪑",
  outside: "🌳",
  social: "💬",
  mind: "🧘",
  tidy: "🧹",
};

export const BREAK_TIPS: readonly BreakTip[] = [
  {
    id: "eyes-20-20-20",
    category: "eyes",
    length: "any",
    pl: "Spójrz przez okno na coś odległego o co najmniej 6 m i patrz tam spokojnie przez 20 sekund.",
    en: "Look out the window at something at least 6 m (20 ft) away and keep your gaze there for 20 seconds.",
    source: "https://www.aao.org/eye-health/diseases/what-is-eye-strain",
  },
  {
    id: "eyes-green-view",
    category: "eyes",
    length: "any",
    pl: "Znajdź za oknem drzewo, trawę albo kawałek nieba i popatrz na nie przez 40 sekund.",
    en: "Find a tree, some grass or a patch of sky outside the window and look at it for 40 seconds.",
    source: "https://doi.org/10.1016/j.jenvp.2015.04.003",
  },
  {
    id: "eyes-slow-blinks",
    category: "eyes",
    length: "short",
    pl: "Zamknij powoli oczy, policz do dwóch i otwórz. Powtórz 10 razy w spokojnym tempie.",
    en: "Slowly close your eyes, count to two and open them. Repeat 10 times at an easy pace.",
    source: "https://www.aao.org/eye-health/diseases/computer-usage",
  },
  {
    id: "eyes-look-around",
    category: "eyes",
    length: "short",
    pl: "Popatrz kolejno w górę, w dół, w lewo i w prawo, a potem zatocz oczami dwa powolne koła w każdą stronę.",
    en: "Look up, down, left and right, then slowly circle your eyes twice in each direction.",
    source: "https://www.ciop.pl/CIOPPortalWAR/file/91726/Ergonomia-pracy-z-komputerem-J-Kaminska.pdf",
  },
  {
    id: "eyes-near-far",
    category: "eyes",
    length: "short",
    pl: "Przez 10 sekund patrz na kciuk wyciągniętej ręki, potem 10 sekund na punkt za oknem. Powtórz 5 razy.",
    en: "Look at your outstretched thumb for 10 seconds, then at a point outside for 10 seconds. Repeat 5 times.",
    source: "https://www.hse.gov.uk/contact/faqs/vdubreaks.htm",
  },
  {
    id: "eyes-screen-free",
    category: "eyes",
    length: "any",
    pl: "Spędź tę przerwę bez ekranów — odłóż też telefon ekranem do dołu.",
    en: "Spend this break away from screens — put your phone face down too.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7044622/",
  },
  {
    id: "stretch-shoulder-rolls",
    category: "stretch",
    length: "any",
    pl: "Zrób 5 powolnych krążeń barkami do tyłu, a potem 5 do przodu.",
    en: "Slowly roll your shoulders backwards 5 times, then forwards 5 times.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-shoulder-shrug",
    category: "stretch",
    length: "short",
    pl: "Unieś barki w stronę uszu, przytrzymaj 3–5 sekund i swobodnie opuść. Powtórz 3 razy.",
    en: "Lift your shoulders towards your ears, hold for 3–5 seconds and let them drop. Repeat 3 times.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-neck-turn",
    category: "stretch",
    length: "any",
    pl: "Usiądź prosto i powoli obróć głowę w lewo, na ile to wygodne. Zostań tak 5 sekund, potem w prawo. 3 razy.",
    en: "Sit tall and slowly turn your head left, as far as is comfortable. Hold for 5 seconds, then go right. 3 times.",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-neck-tilt",
    category: "stretch",
    length: "any",
    pl: "Powoli przechyl głowę uchem w stronę barku i przytrzymaj 10 sekund, jeśli to wygodne. Potem druga strona.",
    en: "Slowly tilt your head, ear towards shoulder, and hold for 10 seconds if comfortable. Then the other side.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-chin-glide",
    category: "stretch",
    length: "short",
    pl: "Siedząc prosto, przesuń głowę poziomo do tyłu, nie unosząc brody. Przytrzymaj 5 sekund, powtórz 5 razy.",
    en: "Sitting tall, glide your head straight back without lifting your chin. Hold for 5 seconds, repeat 5 times.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-chest-open",
    category: "stretch",
    length: "any",
    pl: "Usiądź prosto bez opierania się, ściągnij łopatki i rozłóż ręce na boki. Przytrzymaj 10 s, powtórz 3 razy.",
    en: "Sit upright away from the backrest, draw your shoulder blades together and open your arms wide. Hold 10 s, 3 times.",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-upper-twist",
    category: "stretch",
    length: "any",
    pl: "Skrzyżuj ręce na piersi i obróć tułów w lewo, na ile to wygodne. Zostań tak 5 sekund, potem w prawo. 5 razy.",
    en: "Cross your arms over your chest and turn your upper body left, as far as is comfortable. Hold 5 s, then right. 5 times.",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-side-reach",
    category: "stretch",
    length: "any",
    pl: "Spleć dłonie nad głową, sięgnij w górę, a potem przechyl się w lewo i w prawo, po 10 sekund, jeśli to wygodne.",
    en: "Interlace your fingers overhead, reach up, then lean left and right for 10 seconds each, if comfortable.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-wrists",
    category: "stretch",
    length: "short",
    pl: "Wyciągnij rękę i delikatnie zegnij nadgarstek w górę, potem w dół, po 5 sekund. 3 razy, potem druga ręka.",
    en: "Stretch out one arm and gently bend the wrist up, then down, 5 seconds each. 3 times, then the other hand.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-fingers",
    category: "stretch",
    length: "short",
    pl: "Zaciśnij dłonie w pięści, a potem szeroko rozłóż palce. Powtórz 5 razy, bez pośpiechu.",
    en: "Make fists, then spread your fingers wide. Repeat 5 times, without rushing.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-prayer",
    category: "stretch",
    length: "short",
    pl: "Złóż dłonie przed sobą i powoli opuszczaj nadgarstki do lekkiego rozciągnięcia, nie bólu. Powtórz 3 razy.",
    en: "Press your palms together in front of you and slowly lower your wrists to a light stretch, not pain. 3 times.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-ankles",
    category: "stretch",
    length: "any",
    pl: "Na siedząco wyprostuj nogę nad podłogą i kieruj palce stopy na zmianę od siebie i do siebie. 5 razy na nogę.",
    en: "While seated, straighten one leg off the floor and point your toes away from you and back. 5 times per leg.",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "stretch-standing-back",
    category: "stretch",
    length: "any",
    pl: "Stań z lekko ugiętymi kolanami, dłonie na dole pleców, i odchyl się lekko do tyłu, na ile wygodnie. 3 razy.",
    en: "Stand with knees slightly bent, hands on your lower back, and lean back gently, as far as is comfortable. 3 times.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "stretch-routine",
    category: "stretch",
    length: "long",
    pl: "Zrób spokojny 10-minutowy zestaw: barki, szyja, skręty tułowia, nadgarstki i kostki — w wygodnym zakresie.",
    en: "Do an easy 10-minute set: shoulders, neck, upper-body twists, wrists and ankles — all within a comfortable range.",
    source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
  },
  {
    id: "move-walk-5",
    category: "move",
    length: "any",
    pl: "Wstań i przejdź się 5 minut — po mieszkaniu, korytarzu albo do kuchni i z powrotem.",
    en: "Get up and walk for 5 minutes — around your home, down the corridor or to the kitchen and back.",
    source: "https://www.cuimc.columbia.edu/news/rx-prolonged-sitting-five-minute-stroll-every-half-hour",
  },
  {
    id: "move-stairs",
    category: "move",
    length: "any",
    pl: "Wejdź po schodach piętro wyżej i zejdź, we własnym tempie. Bez schodów? Pospaceruj po korytarzu.",
    en: "Walk up one flight of stairs and back down at your own pace. No stairs? Stroll down the corridor instead.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-march-seated",
    category: "move",
    length: "short",
    pl: "Usiądź prosto i na zmianę unoś kolana jak w marszu, na ile to wygodne — po 5 razy każdą nogą.",
    en: "Sit tall and lift your knees in turn as if marching, as far as is comfortable — 5 times with each leg.",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "move-march-standing",
    category: "move",
    length: "short",
    pl: "Pomaszeruj w miejscu przez minutę, swobodnie machając rękami.",
    en: "March on the spot for a minute, swinging your arms loosely.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-calf-raises",
    category: "move",
    length: "short",
    pl: "Stań przy biurku, oprzyj dłonie o blat i 10 razy powoli wspnij się na palce.",
    en: "Stand at your desk, rest your hands on it and slowly rise onto your toes 10 times.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-sit-to-stand",
    category: "move",
    length: "short",
    pl: "Na stabilnym krześle bez kółek wstań i usiądź 10 razy w spokojnym tempie.",
    en: "On a steady chair without wheels, stand up and sit down 10 times at an easy pace.",
    source: "https://www.nhs.uk/live-well/exercise/strength-and-flexibility-exercises/sitting-exercises/",
  },
  {
    id: "move-one-song",
    category: "move",
    length: "any",
    pl: "Włącz jedną ulubioną piosenkę i ruszaj się do niej, jak masz ochotę — na stojąco lub na siedząco.",
    en: "Put on one favourite song and move to it however you like — standing or seated.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "move-chores",
    category: "move",
    length: "any",
    pl: "Zrób coś ruchowego w domu: rozwieś pranie, podlej kwiaty albo wynieś śmieci.",
    en: "Do something active around the house: hang the laundry, water the plants or take out the rubbish.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC7719906/",
  },
  {
    id: "hydrate-glass",
    category: "hydrate",
    length: "any",
    pl: "Wypij szklankę wody — spokojnie, małymi łykami.",
    en: "Drink a glass of water — slowly, in small sips.",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
  {
    id: "hydrate-refill",
    category: "hydrate",
    length: "short",
    pl: "Przejdź się do kuchni i napełnij butelkę albo kubek wodą na następną sesję.",
    en: "Walk to the kitchen and fill a bottle or mug with water for the next session.",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
  {
    id: "hydrate-in-reach",
    category: "hydrate",
    length: "short",
    pl: "Zanim wrócisz do pracy, postaw szklankę wody w zasięgu ręki.",
    en: "Before you get back to work, put a glass of water within reach.",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
  {
    id: "breathe-count-5",
    category: "breathe",
    length: "any",
    pl: "Usiądź wygodnie. Wdech nosem, licząc do 5, wydech ustami, licząc do 5. Powtarzaj przez 5 minut.",
    en: "Sit comfortably. Breathe in through your nose counting to 5, out through your mouth counting to 5. Keep going for 5 minutes.",
    source: "https://www.nhs.uk/mental-health/self-help/guides-tools-and-activities/breathing-exercises-for-stress/",
  },
  {
    id: "breathe-belly-hand",
    category: "breathe",
    length: "short",
    pl: "Połóż dłoń na brzuchu i przez minutę obserwuj, jak unosi się przy wdechu i opada przy wydechu.",
    en: "Rest a hand on your belly and for one minute notice it rise as you breathe in and fall as you breathe out.",
    source: "https://www.nhs.uk/mental-health/self-help/guides-tools-and-activities/breathing-exercises-for-stress/",
  },
  {
    id: "breathe-long-exhale",
    category: "breathe",
    length: "short",
    pl: "Weź wdech nosem, na końcu dobierz jeszcze trochę powietrza, a potem zrób długi wydech ustami. 5 razy.",
    en: "Breathe in through your nose, top it up with a second short sip of air, then breathe out slowly through your mouth. 5 times.",
    source: "https://pmc.ncbi.nlm.nih.gov/articles/PMC9873947/",
  },
  {
    id: "breathe-open-window",
    category: "breathe",
    length: "short",
    pl: "Otwórz okno i weź 5 spokojnych oddechów: wdech na cztery, wydech na sześć.",
    en: "Open the window and take 5 calm breaths: in for a count of four, out for a count of six.",
    source: "https://www.nhs.uk/mental-health/self-help/guides-tools-and-activities/breathing-exercises-for-stress/",
  },
  {
    id: "breathe-muscle-release",
    category: "breathe",
    length: "long",
    pl: "Napnij mięśnie stóp na 5 sekund i rozluźnij. Przejdź tak w górę ciała, aż do barków i twarzy.",
    en: "Tense your feet for 5 seconds, then let go. Work your way up the body like this to your shoulders and face.",
    source: "https://doi.org/10.1037/ocp0000083",
  },
  {
    id: "posture-reset",
    category: "posture",
    length: "any",
    pl: "Zanim wrócisz: plecy oparte o oparcie, stopy płasko na podłodze, ramiona luźno przy tułowiu.",
    en: "Before you go back: back against the backrest, feet flat on the floor, arms relaxed at your sides.",
    source: "https://ergo.human.cornell.edu/DEA6510/dea6512k/ergo12tips.html",
  },
  {
    id: "posture-screen-check",
    category: "posture",
    length: "short",
    pl: "Sprawdź monitor: górna krawędź na wysokości oczu lub trochę niżej, ekran na długość wyciągniętej ręki.",
    en: "Check your monitor: top edge at or slightly below eye level, screen about an arm's length away.",
    source: "https://www.osha.gov/etools/computer-workstations/components/monitors",
  },
  {
    id: "posture-glare-check",
    category: "posture",
    length: "short",
    pl: "Sprawdź, czy okno albo lampa nie odbijają się w ekranie. Najlepiej, gdy okno jest z boku monitora.",
    en: "Check that no window or lamp reflects in your screen. Ideally the window should be to the side of the monitor.",
    source: "https://www.osha.gov/etools/computer-workstations/workstation-environment",
  },
  {
    id: "posture-shift",
    category: "posture",
    length: "short",
    pl: "Usiądź na chwilę inaczej: przesuń się na krześle, zmień ułożenie nóg, opuść barki.",
    en: "Change how you sit for a while: shift on your chair, move your legs, drop your shoulders.",
    source: "https://www.osha.gov/etools/computer-workstations/work-process",
  },
  {
    id: "outside-step-out",
    category: "outside",
    length: "short",
    pl: "Wyjdź na balkon, przed dom albo stań przy otwartym oknie i pobądź tam 2 minuty.",
    en: "Step out onto a balcony, in front of the building or by an open window and stay there for 2 minutes.",
    source: "https://doi.org/10.1016/j.jenvp.2015.04.003",
  },
  {
    id: "outside-sky",
    category: "outside",
    length: "any",
    pl: "Wyjdź na zewnątrz i popatrz przez minutę na niebo i chmury.",
    en: "Go outside and watch the sky and clouds for a minute.",
    source: "https://doi.org/10.1016/j.jenvp.2015.04.003",
  },
  {
    id: "outside-walk-15",
    category: "outside",
    length: "long",
    pl: "Wyjdź na 15-minutowy spacer, najlepiej tam, gdzie są drzewa albo trawa. Idź spokojnym tempem.",
    en: "Go for a 15-minute walk, ideally somewhere with trees or grass. Keep an easy pace.",
    source: "https://doi.org/10.1037/ocp0000083",
  },
  {
    id: "outside-walk-noticing",
    category: "outside",
    length: "long",
    pl: "Wyjdź na spacer bez telefonu w ręce i licz po drodze drzewa, ptaki albo psy.",
    en: "Go for a walk without your phone in hand and count trees, birds or dogs along the way.",
    source: "https://doi.org/10.1037/ocp0000083",
  },
  {
    id: "social-chat",
    category: "social",
    length: "any",
    pl: "Porozmawiaj chwilę z kimś o czymś zupełnie niezwiązanym z pracą.",
    en: "Have a short chat with someone about something that has nothing to do with work.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "social-message",
    category: "social",
    length: "short",
    pl: "Napisz krótką wiadomość do bliskiej osoby, po prostu żeby się przywitać.",
    en: "Send a short message to someone close, just to say hello.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "social-walk-together",
    category: "social",
    length: "long",
    pl: "Zaproś kogoś na wspólny krótki spacer, nawet wokół budynku.",
    en: "Invite someone for a short walk together, even just around the building.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "social-pet",
    category: "social",
    length: "any",
    pl: "Masz zwierzaka? Poświęć mu kilka minut pełnej uwagi.",
    en: "Got a pet? Give it a few minutes of your full attention.",
    source: null,
  },
  {
    id: "mind-do-nothing",
    category: "mind",
    length: "short",
    pl: "Usiądź wygodnie, zamknij oczy i przez 2 minuty po prostu nic nie rób.",
    en: "Sit comfortably, close your eyes and simply do nothing for 2 minutes.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "mind-one-song",
    category: "mind",
    length: "any",
    pl: "Posłuchaj uważnie jednej piosenki z zamkniętymi oczami, nie robiąc nic innego.",
    en: "Listen closely to one song with your eyes closed, doing nothing else.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "mind-5-4-3",
    category: "mind",
    length: "short",
    pl: "Nazwij w myślach 5 rzeczy, które widzisz, 4, które słyszysz, i 3, których możesz dotknąć.",
    en: "Name in your head 5 things you can see, 4 you can hear and 3 you can touch.",
    source: null,
  },
  {
    id: "mind-wander",
    category: "mind",
    length: "any",
    pl: "Popatrz przez okno i pozwól myślom swobodnie błądzić przez 3 minuty.",
    en: "Look out of the window and let your thoughts wander freely for 3 minutes.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "mind-park-the-task",
    category: "mind",
    length: "short",
    pl: "Zapisz na kartce jedno zdanie: od czego zaczniesz po przerwie. Potem odłóż ten temat.",
    en: "Write one sentence on paper: what you will start with after the break. Then set the topic aside.",
    source: null,
  },
  {
    id: "mind-three-good",
    category: "mind",
    length: "any",
    pl: "Pomyśl o trzech drobnych rzeczach, które dziś poszły dobrze.",
    en: "Think of three small things that went well today.",
    source: null,
  },
  {
    id: "mind-meal-away",
    category: "mind",
    length: "long",
    pl: "Zjedz posiłek z dala od biurka i ekranu — przy stole, na ławce albo w towarzystwie.",
    en: "Eat a meal away from your desk and screen — at a table, on a bench or with company.",
    source: "https://doi.org/10.5465/amj.2011.1072",
  },
  {
    id: "mind-your-choice",
    category: "mind",
    length: "long",
    pl: "Zrób coś, na co naprawdę masz ochotę — ta przerwa jest Twoja.",
    en: "Do something you genuinely feel like doing — this break is yours.",
    source: "https://doi.org/10.5465/amj.2011.1072",
  },
  {
    id: "mind-lie-down",
    category: "mind",
    length: "long",
    pl: "Jeśli możesz, połóż się na 10–15 minut z zamkniętymi oczami. Nastaw budzik.",
    en: "If you can, lie down for 10–15 minutes with your eyes closed. Set an alarm.",
    source: "https://doi.org/10.1002/job.2109",
  },
  {
    id: "tidy-desk",
    category: "tidy",
    length: "short",
    pl: "Uprzątnij biurko: odnieś kubki, zbierz kartki i zostaw tylko to, czego potrzebujesz teraz.",
    en: "Clear your desk: take the mugs away, gather loose papers and keep only what you need right now.",
    source: null,
  },
  {
    id: "tidy-air",
    category: "tidy",
    length: "any",
    pl: "Otwórz szeroko okno na kilka minut i przewietrz pokój.",
    en: "Open the window wide for a few minutes and air the room.",
    source: null,
  },
  {
    id: "tidy-light",
    category: "tidy",
    length: "short",
    pl: "Dopasuj światło: zasłoń okno, jeśli razi, albo zapal lampę, jeśli zrobiło się ciemno.",
    en: "Adjust the light: draw the blind if it's glaring, or switch on a lamp if it's getting dark.",
    source: "https://www.aao.org/eye-health/tips-prevention/digital-devices-your-eyes",
  },
];

/** Good working practices for the idle Skupienie page (one per day). */
export const WORKING_TIPS: readonly WorkingTip[] = [
  {
    id: "gp-screen-position",
    pl: "Ustaw monitor na wprost siebie, ok. 50–70 cm od oczu, z górną krawędzią na wysokości oczu lub trochę niżej.",
    en: "Place the monitor straight ahead, about 50–70 cm from your eyes, with its top edge at or slightly below eye level.",
    source: "https://www.osha.gov/etools/computer-workstations/components/monitors",
  },
  {
    id: "gp-light-and-glare",
    pl: "Ustaw ekran bokiem do okna i dopasuj jego jasność do światła w pokoju.",
    en: "Keep the screen side-on to the window and match its brightness to the light in the room.",
    source: "https://www.osha.gov/etools/computer-workstations/workstation-environment",
  },
  {
    id: "gp-sitting",
    pl: "Plecy na oparciu, stopy płasko na podłodze, łokcie blisko tułowia, nadgarstki w linii z przedramionami.",
    en: "Back against the backrest, feet flat on the floor, elbows close to your body, wrists in line with your forearms.",
    source: "https://ergo.human.cornell.edu/DEA6510/dea6512k/ergo12tips.html",
  },
  {
    id: "gp-short-often",
    pl: "Rób krótkie przerwy często i zanim poczujesz zmęczenie: kilka minut co godzinę to lepiej niż długa przerwa co dwie.",
    en: "Take short breaks often, before you feel tired: a few minutes every hour beats one long break every two hours.",
    source: "https://www.hse.gov.uk/msd/dse/work-routine.htm",
  },
  {
    id: "gp-20-20-20",
    pl: "W trakcie skupienia co ok. 20 minut oderwij wzrok od ekranu i przez 20 sekund patrz w dal. Pamiętaj o mruganiu.",
    en: "While focusing, look away from the screen about every 20 minutes and gaze into the distance for 20 seconds. Remember to blink.",
    source: "https://www.aao.org/eye-health/diseases/what-is-eye-strain",
  },
  {
    id: "gp-change-position",
    pl: "Zmieniaj pozycję: choć raz na godzinę wstań od biurka, nawet na krótką chwilę.",
    en: "Change position: get up from your desk at least once an hour, even briefly.",
    source: "https://www.ciop.pl/CIOPPortalWAR/file/91726/Ergonomia-pracy-z-komputerem-J-Kaminska.pdf",
  },
  {
    id: "gp-batch-messages",
    pl: "Sprawdzaj pocztę i komunikatory o stałych porach, np. w przerwach, a nie po każdym powiadomieniu.",
    en: "Check email and chat at set times, for example during breaks, rather than after every notification.",
    source: "https://doi.org/10.1016/j.chb.2014.11.005",
  },
  {
    id: "gp-single-task",
    pl: "Na jedną sesję wybierz jedno zadanie i zamknij karty oraz okna, których do niego nie potrzebujesz.",
    en: "Pick one task per session and close the tabs and windows you don't need for it.",
    source: "https://www.apa.org/topics/research/multitasking",
  },
  {
    id: "gp-phone-away",
    pl: "Na czas skupienia wycisz telefon i odłóż go poza zasięg wzroku.",
    en: "During focus time, silence your phone and put it out of sight.",
    source: "https://www.ics.uci.edu/~gmark/chi08-mark.pdf",
  },
  {
    id: "gp-water-in-reach",
    pl: "Trzymaj wodę w zasięgu ręki i popijaj ją po trochu przez cały dzień.",
    en: "Keep water within reach and sip it little by little throughout the day.",
    source: "https://www.nhs.uk/live-well/eat-well/food-guidelines-and-food-labels/water-drinks-nutrition/",
  },
];

/** Shown under every stretch tip. */
export const STRETCH_FOOTNOTE: WorkingTip = {
  id: "stretch-footnote",
  pl: "Ruszaj się w wygodnym zakresie: rozciąganie ma być odczuwalne, ale nie bolesne. Coś boli? Pomiń to ćwiczenie.",
  en: "Move within a comfortable range: you should feel a stretch, never pain. If something hurts, skip it.",
  source: "https://www.ccohs.ca/oshanswers/ergonomics/office/stretching.html",
};

/** Polish labour law (screen work breaks): shown only in the Polish UI, never translated. */
export const PL_LEGAL_NOTE = {
  id: "pl-law-note",
  pl: "Pracując przy komputerze co najmniej połowę dnia, masz prawo do 5 minut przerwy po każdej godzinie, wliczanej do czasu pracy.",
  source: "https://isap.sejm.gov.pl/isap.nsf/DocDetails.xsp?id=WDU19981480973",
} as const;

// ---------------------------------------------------------------------------
// Deterministic pick
// ---------------------------------------------------------------------------

/**
 * Breaks shorter than this get `short` (and `any`) tips, longer ones `long` (and `any`) tips. This only sorts the
 * tip catalogue; the break length itself always comes from the session's timing profile (snapshot / popup).
 */
export const SHORT_BREAK_LIMIT_SECS = 10 * 60;

const MS_PER_DAY = 86_400_000;

/** Tips that fit a break of `breakSecs`, in catalogue order. */
export function tipsForBreak(breakSecs: number): BreakTip[] {
  const wanted: TipLength = breakSecs < SHORT_BREAK_LIMIT_SECS ? "short" : "long";
  return BREAK_TIPS.filter((t) => t.length === "any" || t.length === wanted);
}

/** FNV-1a, 32 bit. */
function hashString(text: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

/** mulberry32: small seeded PRNG, 0 <= x < 1. */
function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const orderCache = new Map<string, BreakTip[]>();

/** The session's own shuffled order of the fitting tips (same in every window for the same session). */
function sessionOrder(sessionId: string | null, breakSecs: number): BreakTip[] {
  const pool = tipsForBreak(breakSecs);
  const key = `${sessionId ?? ""}|${pool.length}|${breakSecs < SHORT_BREAK_LIMIT_SECS ? "s" : "l"}`;
  const cached = orderCache.get(key);
  if (cached) return cached;
  const random = seededRandom(hashString(sessionId ?? ""));
  const order = pool.slice();
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    const a = order[i];
    const b = order[j];
    if (a !== undefined && b !== undefined) {
      order[i] = b;
      order[j] = a;
    }
  }
  if (orderCache.size > 16) orderCache.clear();
  orderCache.set(key, order);
  return order;
}

/**
 * The tip for break number `cycle` (1-based, like `SessionSnapshot.cycle` during that break) of a session.
 * Consecutive breaks walk the session's shuffled order, so a break never repeats the previous break's tip and
 * no tip comes back before the whole pool was shown. `offset` steps through it ("Inna propozycja").
 */
export function pickBreakTip(sessionId: string | null, cycle: number, breakSecs: number, offset = 0): BreakTip | null {
  const order = sessionOrder(sessionId, breakSecs);
  const n = order.length;
  if (n === 0) return null;
  const position = Math.max(1, Math.trunc(Number.isFinite(cycle) ? cycle : 1)) - 1 + Math.trunc(offset);
  return order[((position % n) + n) % n] ?? null;
}

/** One working tip per calendar day (local date), the same all day. */
export function workingTipOfDay(date: Date = new Date()): WorkingTip | null {
  const day = Math.floor(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) / MS_PER_DAY);
  return WORKING_TIPS[((day % WORKING_TIPS.length) + WORKING_TIPS.length) % WORKING_TIPS.length] ?? null;
}

/** `https://www.nhs.uk/…` → `nhs.uk` (for the "Źródło" link). */
export function sourceHost(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
