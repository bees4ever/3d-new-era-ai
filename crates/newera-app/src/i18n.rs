//! Interface language. Texts are written in Portuguese in the code and
//! looked up here when another language is chosen; unknown texts stay as
//! written.

use std::sync::atomic::{AtomicU8, Ordering};

/// A language the window speaks.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Lang {
    /// Portuguese (Brazil): the language the texts are written in, so it
    /// needs no table.
    Pt,
    #[default]
    En,
    Es,
    Fr,
    De,
}

impl Lang {
    /// Every language, in the order the menu lists them.
    pub(crate) const ALL: [Self; 5] = [Self::Pt, Self::En, Self::Es, Self::Fr, Self::De];

    /// How the language names itself — the one name whoever looks for it
    /// reads without knowing any of the others.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Pt => "Português (Brasil)",
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::De => "Deutsch",
        }
    }

    /// The two letters a locale name starts with for this language.
    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::Pt => "pt",
            Self::En => "en",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
        }
    }

    /// Whether numbers are written with a comma for the decimals: everyone
    /// here but English.
    pub(crate) const fn decimal_comma(self) -> bool {
        !matches!(self, Self::En)
    }

    /// Which side of a table row the language is on; Portuguese is the key
    /// of the row, not a side of it.
    const fn column(self) -> Option<usize> {
        match self {
            Self::Pt => None,
            Self::En => Some(0),
            Self::Es => Some(1),
            Self::Fr => Some(2),
            Self::De => Some(3),
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Pt => 0,
            Self::En => 1,
            Self::Es => 2,
            Self::Fr => 3,
            Self::De => 4,
        }
    }

    const fn from_code(code: u8) -> Self {
        match code {
            1 => Self::En,
            2 => Self::Es,
            3 => Self::Fr,
            4 => Self::De,
            _ => Self::Pt,
        }
    }

    /// The language a locale name asks for — `pt_BR.UTF-8`, `fr-CA`, `es` —
    /// or nothing when the window does not speak it.
    fn from_locale(locale: &str) -> Option<Self> {
        let tag = locale.get(..2)?.to_ascii_lowercase();
        Self::ALL.into_iter().find(|lang| lang.tag() == tag)
    }

    /// What the system asks for, when the window speaks it. `NEWERA_LANG`
    /// comes first, as it does in the installers.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_system() -> Option<Self> {
        ["NEWERA_LANG", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find_map(|locale| Self::from_locale(&locale))
    }

    /// In the browser there is no environment to read: the language is the
    /// one the browser itself is set to.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_system() -> Option<Self> {
        Self::from_locale(&web_sys::window()?.navigator().language()?)
    }
}

static LANG: AtomicU8 = AtomicU8::new(0);

/// Switches the interface language.
pub(crate) fn set(lang: Lang) {
    LANG.store(lang.code(), Ordering::Relaxed);
}

pub(crate) fn lang() -> Lang {
    Lang::from_code(LANG.load(Ordering::Relaxed))
}

/// The language a finding is said in. Findings carry Portuguese and English;
/// where the window speaks neither, English is the one more readers share.
pub(crate) fn findings_language() -> newera_core::vocabulary::Language {
    match lang() {
        Lang::Pt => newera_core::vocabulary::Language::Portuguese,
        Lang::En | Lang::Es | Lang::Fr | Lang::De => newera_core::vocabulary::Language::English,
    }
}

/// Whether numbers on screen take a comma for the decimals.
pub(crate) fn decimal_comma() -> bool {
    lang().decimal_comma()
}

/// The text in the current language.
/// Translates a line that only exists at run time: what a background job says
/// it is doing comes from the work itself, not from a literal in this crate.
pub(crate) fn dynamic(pt: &str) -> String {
    let Some(column) = lang().column() else {
        return pt.to_owned();
    };
    translations(pt).map_or_else(|| pt.to_owned(), |row| row[column].to_owned())
}

/// A name that lives in the project, as the window shows it. The defaults
/// the project starts with ("Nova casa", "Versão 1", "Versão 1 (cópia)") and
/// the catalog names pieces are placed under are Portuguese in the file; on
/// screen they follow the window's language. A name somebody typed is theirs
/// and stays as it is.
pub(crate) fn shown(name: &str) -> String {
    shown_in(name, lang())
}

/// [`shown`] in one named language. See [`say`].
fn shown_in(name: &str, lang: Lang) -> String {
    let Some(column) = lang.column() else {
        return name.to_owned();
    };
    if let Some(original) = name.strip_suffix(" (cópia)") {
        return format!("{}{}", shown_in(original, lang), say(" (cópia)", lang));
    }
    if let Some(n) = name.strip_prefix("Versão ")
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        return fill_in("Versão {}", lang, &[&n]);
    }
    if let Some(n) = name.strip_suffix("º andar")
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        return fill_in("{}º andar", lang, &[&n]);
    }
    translations(name).map_or_else(|| name.to_owned(), |row| row[column].to_owned())
}

/// The text in the language the window is set to: `pt` is the Portuguese
/// the code is written in, and comes back as it is when there is no
/// translation for it.
pub(crate) fn tr(pt: &'static str) -> &'static str {
    say(pt, lang())
}

/// The text in every language the window speaks, Portuguese first: what a
/// name typed in any of them is matched against.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn every(pt: &'static str) -> impl Iterator<Item = &'static str> {
    Lang::ALL.into_iter().map(move |lang| say(pt, lang))
}

/// The text in one named language, whatever the window is set to. The window
/// itself always wants [`tr`]; this is for the tests, which run beside each
/// other and must not shout across.
fn say(pt: &'static str, lang: Lang) -> &'static str {
    let Some(column) = lang.column() else {
        return pt;
    };
    translations(pt).map_or(pt, |row| row[column])
}

/// A sentence with things in it: `{}` in the text marks where each one goes,
/// in every language, so a translation is free to put them elsewhere.
pub(crate) fn fill(pt: &'static str, parts: &[&dyn std::fmt::Display]) -> String {
    fill_in(pt, lang(), parts)
}

/// [`fill`] in one named language. See [`say`].
fn fill_in(pt: &'static str, lang: Lang, parts: &[&dyn std::fmt::Display]) -> String {
    let mut text = String::new();
    let mut rest = say(pt, lang);
    for part in parts {
        let Some((before, after)) = rest.split_once("{}") else {
            break;
        };
        text.push_str(before);
        text.push_str(&part.to_string());
        rest = after;
    }
    text.push_str(rest);
    text
}

/// What the text says in English, Spanish, French and German, in this order.
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
fn translations(pt: &str) -> Option<[&'static str; 4]> {
    Some(match pt {
        "Medições de esmog" => [
            "Electrosmog readings",
            "Mediciones de electrosmog",
            "Mesures d'électrosmog",
            "Esmog - Messwerte",
        ],
        "Clique os cantos e duplo clique fecha · altura e valor no diálogo" => [
            "Click the corners and double-click to close · height and reading in the dialog",
            "Pulse las esquinas y doble clic para cerrar · altura y valor en el diálogo",
            "Cliquez les coins et double-clic pour fermer · hauteur et mesure dans la boîte",
            "Ecken klicken und Doppelklick schließt · Höhe und Messwert im Dialog",
        ],
        "Modificar medição de esmog" => [
            "Modify electrosmog reading",
            "Modificar medición de electrosmog",
            "Modifier la mesure d'électrosmog",
            "Esmog-Messwert ändern",
        ],
        "Valor medido" => ["Reading", "Valor medido", "Valeur mesurée", "Messwert"],

        " · arco" => [" · arc", " · arco", " · arc", " · Bogen"],
        " (cópia)" => [" (copy)", " (copia)", " (copie)", " (Kopie)"],
        "Nova casa" => ["New home", "Casa nueva", "Nouvelle maison", "Neues Haus"],
        "Versão {}" => ["Version {}", "Versión {}", "Version {}", "Version {}"],
        "A tarefa termina sozinha em segundo plano; o resultado é descartado." => [
            "The task finishes on its own in the background; its result is thrown away.",
            "La tarea termina sola en segundo plano; su resultado se descarta.",
            "La tâche se termine seule en arrière-plan ; son résultat est ignoré.",
            "Die Aufgabe läuft im Hintergrund zu Ende; das Ergebnis wird verworfen.",
        ],
        "Abrindo imagens e modelos" => [
            "Opening images and models",
            "Abriendo imágenes y modelos",
            "Ouverture des images et des modèles",
            "Bilder und Modelle werden geöffnet",
        ],
        "Abrindo projeto" => [
            "Opening project",
            "Abriendo proyecto",
            "Ouverture du projet",
            "Projekt wird geöffnet",
        ],
        "Desenhando a planta" => ["Drawing the plan", "Dibujando el plano", "Tracé du plan", "Grundriss wird gezeichnet"],
        "Exportando" => ["Exporting", "Exportando", "Exportation", "Export"],
        "Extraindo imagens e modelos" => [
            "Extracting images and models",
            "Extrayendo imágenes y modelos",
            "Extraction des images et des modèles",
            "Bilder und Modelle werden entpackt",
        ],
        "Forçar e encerrar a espera" => [
            "Force it and stop waiting",
            "Forzar y dejar de esperar",
            "Forcer et arrêter d'attendre",
            "Erzwingen und nicht weiter warten",
        ],
        "Gravando o projeto" => [
            "Writing the project",
            "Grabando el proyecto",
            "Écriture du projet",
            "Projekt wird geschrieben",
        ],
        "Guardando imagens e modelos" => [
            "Storing images and models",
            "Guardando imágenes y modelos",
            "Enregistrement des images et des modèles",
            "Bilder und Modelle werden gespeichert",
        ],
        "Importando modelo" => [
            "Importing model",
            "Importando modelo",
            "Importation du modèle",
            "Modell wird importiert",
        ],
        "Lendo o arquivo" => [
            "Reading the file",
            "Leyendo el archivo",
            "Lecture du fichier",
            "Datei wird gelesen",
        ],
        "Lendo o projeto" => [
            "Reading the project",
            "Leyendo el proyecto",
            "Lecture du projet",
            "Projekt wird gelesen",
        ],
        "Montando o modelo 3D" => [
            "Building the 3D model",
            "Montando el modelo 3D",
            "Construction du modèle 3D",
            "3D-Modell wird gebaut",
        ],
        "Parando…" => ["Stopping…", "Deteniendo…", "Arrêt…", "Wird beendet…"],
        "Salvando projeto" => [
            "Saving project",
            "Guardando proyecto",
            "Enregistrement du projet",
            "Projekt wird gespeichert",
        ],
        "Trabalhando…" => ["Working…", "Trabajando…", "Travail en cours…", "Arbeitet…"],
        "⚠ Espera encerrada — a tarefa termina em segundo plano." => [
            "⚠ Stopped waiting — the task finishes in the background.",
            "⚠ Espera terminada — la tarea sigue en segundo plano.",
            "⚠ Attente interrompue — la tâche se termine en arrière-plan.",
            "⚠ Warten beendet — die Aufgabe läuft im Hintergrund zu Ende.",
        ],
        "Ajustar ao telhado" => ["Fit to the roof", "Ajustar al tejado", "Ajuster au toit", "An das Dach anpassen"],
        "Código de obras" => [
            "Building code",
            "Código de obras",
            "Code de la construction",
            "Bauvorschrift",
        ],
        "Embutir peça no móvel selecionado" => [
            "Embed the piece in the selected joinery",
            "Empotrar la pieza en el mueble seleccionado",
            "Encastrer la pièce dans le meuble sélectionné",
            "Das Stück in die gewählte Schreinerei einbauen",
        ],
        "Embutido:" => ["Embedded:", "Empotrado:", "Encastré :", "Eingebaut:"],
        "Armários na parede" => [
            "Cabinets on the wall",
            "Armarios en la pared",
            "Meubles sur le mur",
            "Schränke an der Wand",
        ],
        "Armários na parede…" => [
            "Cabinets on the wall…",
            "Armarios en la pared…",
            "Meubles sur le mur…",
            "Schränke an der Wand…",
        ],
        "Balcões" => ["Base cabinets", "Muebles bajos", "Meubles bas", "Unterschränke"],
        "Aéreos" => ["Wall cabinets", "Muebles altos", "Meubles hauts", "Oberschränke"],
        "Entre a norma e a lei do município, prevalece o mais restritivo." => [
            "Between the standard and the city's law, the more restrictive one wins.",
            "Entre la norma y la ley del municipio, prevalece la más restrictiva.",
            "Entre la norme et la loi de la commune, la plus restrictive l'emporte.",
            "Zwischen Norm und städtischem Recht gilt die strengere Regel.",
        ],
        "Fontes desta revisão:" => [
            "Sources of this review:",
            "Fuentes de esta revisión:",
            "Sources de cette révision :",
            "Quellen dieser Prüfung:",
        ],
        "Não informado" => ["Not set", "Sin definir", "Non défini", "Nicht gesetzt"],
        "Referências brasileiras onde existe norma; áreas e janelas variam com o código de obras do município." => [
            "Brazilian standards where one exists; areas and windows vary with the city's building code.",
            "Referencias brasileñas donde existe norma; áreas y ventanas varían según el código de obras del municipio.",
            "Normes brésiliennes lorsqu'elles existent ; surfaces et fenêtres varient selon le code de la construction de la commune.",
            "Brasilianische Normen, wo es sie gibt; Flächen und Fenster richten sich nach der städtischen Bauordnung.",
        ],
        "Torres / guarda-roupa" => [
            "Tall / wardrobe",
            "Columnas / armario ropero",
            "Colonnes / dressing",
            "Hochschrank / Garderobe",
        ],
        "Pia centrada em" => [
            "Sink centered at",
            "Fregadero centrado en",
            "Évier centré à",
            "Spüle mittig bei",
        ],
        "Cooktop centrado em" => [
            "Cooktop centered at",
            "Placa de cocción centrada en",
            "Table de cuisson centrée à",
            "Kochfeld mittig bei",
        ],
        "descreve" => ["describes", "describe", "décrit", "beschreibt"],
        "do início da parede" => [
            "from the wall start",
            "desde el inicio de la pared",
            "depuis le début du mur",
            "vom Wandbeginn",
        ],
        "Bancada de pedra por cima" => [
            "Stone countertop on top",
            "Encimera de piedra encima",
            "Plan de travail en pierre au-dessus",
            "Stein-Arbeitsplatte obenauf",
        ],
        "Frentes" => ["Fronts", "Frentes", "Façades", "Fronten"],
        "Gaveteiros" => ["Drawer units", "Cajoneras", "Blocs-tiroirs", "Schubkästen"],
        "automático" => ["automatic", "automático", "automatique", "automatisch"],
        "Mede o trecho livre entre cantos, portas, janelas e eletrodomésticos e divide em módulos sem sobras; substitui os armários que já estão nessa parede." => [
            "Measures the free stretch between corners, doors, windows and appliances and splits it into modules with no leftovers; replaces the cabinets already on that wall.",
            "Mide el tramo libre entre esquinas, puertas, ventanas y electrodomésticos y lo divide en módulos sin sobras; sustituye los armarios que ya están en esa pared.",
            "Mesure la portion libre entre angles, portes, fenêtres et appareils, puis la divise en modules sans chute ; remplace les meubles déjà posés sur ce mur.",
            "Misst die freie Strecke zwischen Ecken, Türen, Fenstern und Geräten und teilt sie in Module ohne Rest; ersetzt die Schränke, die schon an dieser Wand stehen.",
        ],
        "Pré-visualizar" => ["Preview", "Vista previa", "Aperçu", "Vorschau"],
        "Criar armários" => ["Build cabinets", "Crear armarios", "Créer les meubles", "Schränke bauen"],
        "Criado (Ctrl+Z desfaz):" => [
            "Built (Ctrl+Z undoes it):",
            "Creado (Ctrl+Z deshace):",
            "Créé (Ctrl+Z annule) :",
            "Gebaut (Strg+Z macht es rückgängig):",
        ],
        "Prévia:" => ["Preview:", "Vista previa:", "Aperçu :", "Vorschau:"],
        "doutrina" => ["doctrine", "doctrina", "doctrine", "Lehre"],
        "mede" => ["measures", "mide", "mesure", "Maßnahmen"],
        "obriga" => ["obliges", "obliga", "impose", "verpflichtet"],
        "portas" => ["doors", "puertas", "portes", "Türen"],
        "gaveteiro" => ["drawers", "cajonera", "tiroirs", "Schubladen"],
        "porta-temperos" => ["pull-out", "especiero", "coulissant", "Auszug"],
        "canto cego" => ["blind corner", "esquina ciega", "angle borgne", "Eckschrank"],
        "cabideiro" => ["hanging", "barra de colgar", "penderie", "hängend"],
        "referencia" => ["references", "referencia", "référence", "Referenzen"],
        "sobre a geladeira" => [
            "over the fridge",
            "sobre el frigorífico",
            "au-dessus du réfrigérateur",
            "über dem Kühlschrank",
        ],
        "pia" => ["sink", "fregadero", "évier", "Spüle"],
        "tamponamento" => ["filler", "panel de relleno", "fileur", "Füllstück"],
        "bancada" => ["countertop", "encimera", "plan de travail", "Arbeitsplatte"],
        "Ergonomia" => ["Ergonomics", "Ergonomía", "Ergonomie", "Ergonomie"],
        "Ergonomia…" => ["Ergonomics…", "Ergonomía…", "Ergonomie…", "Ergonomie…"],
        "Moradores" => ["Occupants", "Habitantes", "Occupants", "Bewohner"],
        "Crianças" => ["Children", "Niños", "Enfants", "Kinder"],
        "Idosos" => ["Elderly", "Mayores", "Personnes âgées", "Ältere Menschen"],
        "Altura de quem cozinha" => [
            "Cook's height",
            "Altura de quien cocina",
            "Taille de la personne qui cuisine",
            "Körpergröße der kochenden Person",
        ],
        "Alguém usa cadeira de rodas (NBR 9050)" => [
            "Someone uses a wheelchair (NBR 9050)",
            "Alguien usa silla de ruedas (NBR 9050)",
            "Une personne en fauteuil roulant (NBR 9050)",
            "Jemand nutzt einen Rollstuhl (NBR 9050)",
        ],
        "Incluir elétrica na nota" => [
            "Include electrical in score",
            "Incluir electricidad en la nota",
            "Inclure l’électricité dans la note",
            "Elektro in die Bewertung einbeziehen",
        ],
        "Incluir hidráulica na nota" => [
            "Include plumbing in score",
            "Incluir fontanería en la nota",
            "Inclure la plomberie dans la note",
            "Sanitär in die Bewertung einbeziehen",
        ],
        "Pendências fora do escopo continuam visíveis. A nota não certifica o projeto." => [
            "Findings outside the scope remain visible. The score does not certify the project.",
            "Los hallazgos fuera del alcance siguen visibles. La nota no certifica el proyecto.",
            "Les constats hors périmètre restent visibles. La note ne certifie pas le projet.",
            "Hinweise außerhalb des Rahmens bleiben sichtbar. Die Note zertifiziert das Projekt nicht.",
        ],
        "Fora do escopo da nota" => [
            "Outside score scope",
            "Fuera del alcance de la nota",
            "Hors périmètre de la note",
            "Außerhalb der Bewertung",
        ],
        "nota de habitabilidade" => [
            "habitability score",
            "nota de habitabilidad",
            "note d'habitabilité",
            "Bewohnbarkeitsnote",
        ],
        "lugares para dormir" => ["beds", "plazas para dormir", "couchages", "Betten"],
        "banheiros" => ["bathrooms", "baños", "salles de bain", "Bäder"],
        "à mesa" => ["at the table", "en la mesa", "à table", "am Tisch"],
        "na sala" => ["in the living room", "en el salón", "au salon", "im Wohnzimmer"],
        "de guarda-roupa" => ["of wardrobe", "de armario ropero", "de dressing", "an Garderobe"],
        "Nada a apontar para esses moradores." => [
            "Nothing to point out for these occupants.",
            "Nada que señalar para estos habitantes.",
            "Rien à signaler pour ces occupants.",
            "Für diese Bewohner gibt es nichts anzumerken.",
        ],
        "Selecionar na planta" => [
            "Select on the plan",
            "Seleccionar en el plano",
            "Sélectionner sur le plan",
            "Im Grundriss wählen",
        ],
        "Aplicar correção" => ["Apply fix", "Aplicar corrección", "Appliquer la correction", "Korrektur anwenden"],
        "Correção aplicada (Ctrl+Z desfaz)." => [
            "Fix applied (Ctrl+Z undoes it).",
            "Corrección aplicada (Ctrl+Z deshace).",
            "Correction appliquée (Ctrl+Z annule).",
            "Korrektur angewendet (Strg+Z macht sie rückgängig).",
        ],
        "Referências: NBR 9050, NBR 15575-1 e IBGE; áreas e janelas variam com o código de obras do município." => [
            "References: NBR 9050, NBR 15575-1 and IBGE; areas and windows vary with the city's building code.",
            "Referencias: NBR 9050, NBR 15575-1 e IBGE; áreas y ventanas varían según el código de obras del municipio.",
            "Références : NBR 9050, NBR 15575-1 et IBGE ; surfaces et fenêtres varient selon le code de la construction de la commune.",
            "Quellen: NBR 9050, NBR 15575-1 und IBGE; Flächen und Fenster richten sich nach der städtischen Bauordnung.",
        ],
        " · luz" => [" · light", " · luz", " · lumière", " · Licht"],
        "(cópia)" => ["(copy)", "(copia)", "(copie)", "(Kopie)"],
        "Abrir" => ["Open", "Abrir", "Ouvrir", "Öffnen"],
        "Abrir (Ctrl+O)" => ["Open (Ctrl+O)", "Abrir (Ctrl+O)", "Ouvrir (Ctrl+O)", "Öffnen (Strg+O)"],
        "Abrir link" => ["Open link", "Abrir enlace", "Ouvrir le lien", "Link öffnen"],
        "Abrir…" => ["Open…", "Abrir…", "Ouvrir…", "Öffnen…"],
        "Adicionar texto" => ["Add text", "Añadir texto", "Ajouter du texte", "Text hinzufügen"],
        "Adicionar um andar acima do mais alto" => [
            "Add a storey above the highest one",
            "Añadir una planta encima de la más alta",
            "Ajouter un étage au-dessus du plus haut",
            "Ein Geschoss über dem obersten hinzufügen",
        ],
        "Afastamento" => ["Offset", "Separación", "Décalage", "Abstand"],
        "Afastar (Ctrl -)" => [
            "Zoom out (Ctrl -)",
            "Alejar (Ctrl -)",
            "Zoom arrière (Ctrl -)",
            "Verkleinern (Strg -)",
        ],
        "Ajuda" => ["Help", "Ayuda", "Aide", "Hilfe"],
        "Quadro de cargas e NBR 5410" => [
            "Load schedule and NBR 5410",
            "Cuadro de cargas y NBR 5410",
            "Tableau des charges et NBR 5410",
            "Lastplan und NBR 5410",
        ],
        "Nenhum circuito: atribua os pontos a circuitos (MCP electrical assign)." => [
            "No circuits: assign the points to circuits (MCP electrical assign).",
            "Ningún circuito: asigne los puntos a circuitos (MCP electrical assign).",
            "Aucun circuit : affectez les points à des circuits (MCP electrical assign).",
            "Keine Stromkreise: Punkte Stromkreisen zuweisen (MCP electrical assign).",
        ],
        "Circuito" => ["Circuit", "Circuito", "Circuit", "Stromkreis"],
        "Pontos" => ["Points", "Puntos", "Points", "Punkte"],
        "VA" => ["VA", "VA", "VA", "VA"],
        "V" => ["V", "V", "V", "V"],
        "A" => ["A", "A", "A", "A"],
        "Fio mm²" => ["Wire mm²", "Hilo mm²", "Fil mm²", "Leitung mm²"],
        "Disjuntor" => ["Breaker", "Magnetotérmico", "Disjoncteur", "Sicherung"],
        "DR" => ["RCD", "Diferencial", "Différentiel", "FI"],
        "Total instalado:" => ["Installed total:", "Total instalado:", "Total installé :", "Installiert gesamt:"],
        "Nada a apontar." => [
            "Nothing to point out.",
            "Nada que señalar.",
            "Rien à signaler.",
            "Nichts anzumerken.",
        ],
        "Camadas" => ["Layers", "Capas", "Calques", "Ebenen"],
        "Iluminação" => ["Lighting", "Iluminación", "Éclairage", "Beleuchtung"],
        "Eletrodomésticos" => ["Appliances", "Electrodomésticos", "Appareils", "Geräte"],
        "Marcenaria" => ["Joinery", "Carpintería", "Menuiserie", "Schreinerei"],
        "Mostrar ou esconder na planta e no 3D" => [
            "Show or hide on the plan and in 3D",
            "Mostrar u ocultar en el plano y en 3D",
            "Afficher ou masquer sur le plan et en 3D",
            "Im Grundriss und in 3D ein- oder ausblenden",
        ],
        "Mostrar tudo no 3D" => [
            "Show everything in 3D",
            "Mostrar todo en 3D",
            "Tout afficher en 3D",
            "In 3D alles zeigen",
        ],
        "Ligado, o 3D mostra a obra inteira; desligado, esconde o que a planta esconde" => [
            "On, the 3D shows the whole building; off, it hides what the plan hides",
            "Activado, el 3D muestra la obra entera; desactivado, oculta lo que el plano oculta",
            "Activé, la 3D montre tout le bâtiment ; désactivé, elle masque ce que le plan masque",
            "An: die 3D-Ansicht zeigt das ganze Gebäude; aus: sie verbirgt, was der Grundriss verbirgt",
        ],
        "Enviar relatórios de erro" => [
            "Send error reports",
            "Enviar informes de error",
            "Envoyer les rapports d'erreur",
            "Fehlerberichte senden",
        ],
        "Falhas e notas de uso vão para os desenvolvedores, sem o seu projeto, sem IP e sem nome da máquina." => [
            "Crashes and usage notes go to the developers, without your project, your IP or your machine's name.",
            "Los fallos y las notas de uso van a los desarrolladores, sin su proyecto, sin IP y sin el nombre de la máquina.",
            "Les plantages et les notes d'usage vont aux développeurs, sans votre projet, sans IP ni nom de machine.",
            "Abstürze und Nutzungshinweise gehen an die Entwickler, ohne Ihr Projekt, Ihre IP oder den Namen Ihres Rechners.",
        ],
        "Ajustes…" => ["Settings…", "Ajustes…", "Réglages…", "Einstellungen…"],
        "Alinhamento" => ["Alignment", "Alineación", "Alignement", "Ausrichtung"],
        "Altura" => ["Height", "Altura", "Hauteur", "Höhe"],
        "Andar" => ["Storey", "Planta", "Étage", "Geschoss"],
        "Aproximar (Ctrl +)" => [
            "Zoom in (Ctrl +)",
            "Acercar (Ctrl +)",
            "Zoom avant (Ctrl +)",
            "Vergrößern (Strg +)",
        ],
        "Apoiar o projeto" => [
            "Support the project",
            "Apoyar el proyecto",
            "Soutenir le projet",
            "Das Projekt unterstützen",
        ],
        "Apoiar no Buy Me a Coffee" => [
            "Support on Buy Me a Coffee",
            "Apoyar en Buy Me a Coffee",
            "Soutenir sur Buy Me a Coffee",
            "Auf Buy Me a Coffee unterstützen",
        ],
        "Abrir o GitHub" => ["Open GitHub", "Abrir GitHub", "Ouvrir GitHub", "GitHub öffnen"],
        "Agora não" => ["Not now", "Ahora no", "Pas maintenant", "Jetzt nicht"],
        "Nova versão {} disponível" => [
            "New version {} available",
            "Nueva versión {} disponible",
            "Nouvelle version {} disponible",
            "Neue Version {} verfügbar",
        ],
        "Muita gente usa o 3D New Era AI todos os dias, mas quase ninguém apoia o projeto." => [
            "Many people use 3D New Era AI every day, but almost no one supports the project.",
            "Mucha gente usa 3D New Era AI todos los días, pero casi nadie apoya el proyecto.",
            "Beaucoup de gens utilisent 3D New Era AI chaque jour, mais presque personne ne soutient le projet.",
            "Viele nutzen 3D New Era AI täglich, aber fast niemand unterstützt das Projekt.",
        ],
        "As IAs que usam a ferramenta enviam sozinhas relatos de problemas e sugestões de melhoria, e eles chegam em grande número. Quero corrigir todos, mas me falta tempo e dinheiro para pagar as ferramentas de IA e os tokens que esse trabalho exige." => [
            "The AIs that use the tool send problem reports and improvement suggestions on their own, and they arrive in large numbers. I want to fix them all, but I lack the time and the money to pay for the AI tools and tokens this work takes.",
            "Las IA que usan la herramienta envían solas reportes de problemas y sugerencias de mejora, y llegan en gran número. Quiero corregirlos todos, pero me falta tiempo y dinero para pagar las herramientas de IA y los tokens que este trabajo exige.",
            "Les IA qui utilisent l'outil envoient d'elles-mêmes des rapports de problèmes et des suggestions d'amélioration, et ils arrivent en grand nombre. Je veux tous les corriger, mais il me manque le temps et l'argent pour payer les outils d'IA et les tokens que ce travail demande.",
            "Die KIs, die das Werkzeug nutzen, schicken von selbst Fehlerberichte und Verbesserungsvorschläge, und sie kommen in großer Zahl. Ich möchte sie alle beheben, aber mir fehlen Zeit und Geld für die KI-Werkzeuge und Tokens, die diese Arbeit braucht.",
        ],
        "Se a ferramenta é útil para você, considere apoiar no Buy Me a Coffee." => [
            "If the tool is useful to you, please consider supporting it on Buy Me a Coffee.",
            "Si la herramienta te resulta útil, considera apoyarla en Buy Me a Coffee.",
            "Si l'outil vous est utile, pensez à le soutenir sur Buy Me a Coffee.",
            "Wenn das Werkzeug nützlich ist, unterstützen Sie es bitte auf Buy Me a Coffee.",
        ],
        "Também dá para ajudar com código: abra issues e pull requests no GitHub. Todos serão avaliados, e o que for aceito entra na próxima versão." => [
            "You can also help with code: open issues and pull requests on GitHub. Every one will be reviewed, and what is accepted goes into the next release.",
            "También puedes ayudar con código: abre issues y pull requests en GitHub. Todos serán evaluados, y lo que se acepte entra en la próxima versión.",
            "Vous pouvez aussi aider avec du code : ouvrez des issues et des pull requests sur GitHub. Toutes seront examinées, et ce qui est accepté entre dans la prochaine version.",
            "Sie können auch mit Code helfen: Issues und Pull Requests auf GitHub öffnen. Jedes wird gelesen, und was angenommen wird, kommt in die nächste Version.",
        ],
        "Arco" => ["Arc", "Arco", "Arc", "Bogen"],
        "Arquitetura" => ["Architecture", "Arquitectura", "Architecture", "Architektur"],
        "Arquivo" => ["File", "Archivo", "Fichier", "Datei"],
        "Arraste para mover a vista" => [
            "Drag to move the view",
            "Arrastre para mover la vista",
            "Faites glisser pour déplacer la vue",
            "Ziehen, um die Ansicht zu verschieben",
        ],
        "Arraste: girar · Shift/botão do meio: mover · Scroll: zoom · F: enquadrar" => [
            "Drag: orbit · Shift/middle button: pan · Scroll: zoom · F: frame",
            "Arrastrar: girar · Mayús/botón central: mover · Rueda: zoom · F: encuadrar",
            "Glisser : pivoter · Maj/bouton du milieu : déplacer · Molette : zoom · F : cadrer",
            "Ziehen: umkreisen · Umschalt/Mitteltaste: verschieben · Scrollen: zoomen · F: einpassen",
        ],
        "Atalhos e ferramentas" => [
            "Shortcuts and tools",
            "Atajos y herramientas",
            "Raccourcis et outils",
            "Tastenkürzel und Werkzeuge",
        ],
        "Automático (símbolos e vista de cima dos modelos)" => [
            "Automatic (symbols, top views for models)",
            "Automático (símbolos y vista superior de los modelos)",
            "Automatique (symboles, vues de dessus pour les modèles)",
            "Automatisch (Symbole, Draufsichten für Modelle)",
        ],
        "Autor" => ["Author", "Autor", "Auteur", "Autor"],
        "Boa" => ["Good", "Buena", "Bonne", "Gut"],
        "Buscar: cama, janela, sofá…" => [
            "Search: bed, window, sofa…",
            "Buscar: cama, ventana, sofá…",
            "Rechercher : lit, fenêtre, canapé…",
            "Suche: Bett, Fenster, Sofa…",
        ],
        "Calibrar e posicionar" => [
            "Calibrate and place",
            "Calibrar y posicionar",
            "Calibrer et placer",
            "Kalibrieren und platzieren",
        ],
        "Calibrar escala da imagem" => [
            "Calibrate image scale",
            "Calibrar escala de la imagen",
            "Calibrer l'échelle de l'image",
            "Bildmaßstab kalibrieren",
        ],
        "Cancelar" => ["Cancel", "Cancelar", "Annuler", "Abbrechen"],
        "Casa e bússola" => ["Home and compass", "Casa y brújula", "Maison et boussole", "Haus und Kompass"],
        "Casa e bússola…" => [
            "Home and compass…",
            "Casa y brújula…",
            "Maison et boussole…",
            "Haus und Kompass…",
        ],
        "Catálogo" => ["Catalog", "Catálogo", "Catalogue", "Katalog"],
        "Catálogo de origem" => [
            "Source catalog",
            "Catálogo de origen",
            "Catalogue d'origine",
            "Quellkatalog",
        ],
        "Centro" => ["Center", "Centro", "Centre", "Mitte"],
        "Centro (x, y)" => ["Center (x, y)", "Centro (x, y)", "Centre (x, y)", "Mitte (x, y)"],
        "Clique dois pontos de medida conhecida · arraste para posicionar a imagem" => [
            "Click two points of a known distance · drag to place the image",
            "Haga clic en dos puntos de distancia conocida · arrastre para posicionar la imagen",
            "Cliquez deux points d'une distance connue · faites glisser pour placer l'image",
            "Zwei Punkte mit bekanntem Abstand klicken · ziehen, um das Bild zu legen",
        ],
        "Clique e depois clique na planta para posicionar" => [
            "Click, then click on the plan to place",
            "Haga clic y luego en el plano para posicionar",
            "Cliquez, puis cliquez sur le plan pour placer",
            "Klicken, dann im Grundriss klicken zum Platzieren",
        ],
        "Clique encadeia paredes · digite o comprimento + Enter · Shift desliga o ímã · duplo clique encerra" => [
            "Clicks chain walls · type a length + Enter · Shift disables magnetism · double-click ends",
            "El clic encadena paredes · escriba la longitud + Enter · Mayús desactiva el imán · doble clic termina",
            "Les clics enchaînent les murs · tapez une longueur + Entrée · Maj désactive l'aimant · double-clic termine",
            "Klicks ketten Wände · Länge tippen + Enter · Umschalt schaltet den Magneten aus · Doppelklick beendet",
        ],
        "Clique início e fim, mova para afastar e clique · duplo clique numa parede cota a parede" => [
            "Click start and end, move to offset and click · double-click a wall to dimension it",
            "Haga clic en inicio y fin, mueva para separar y haga clic · doble clic en una pared la acota",
            "Cliquez début et fin, déplacez pour décaler et cliquez · double-clic sur un mur pour le coter",
            "Anfang und Ende klicken, zum Abstand ziehen und klicken · Doppelklick auf eine Wand bemaßt sie",
        ],
        "Clique onde o texto deve ficar" => [
            "Click where the text goes",
            "Haga clic donde debe quedar el texto",
            "Cliquez à l'endroit où placer le texte",
            "Klicken, wo der Text stehen soll",
        ],
        "Clique os cantos e duplo clique fecha · duplo clique dentro de paredes detecta o cômodo" => [
            "Click the corners and double-click to close · double-click inside walls detects the room",
            "Haga clic en las esquinas y doble clic para cerrar · doble clic dentro de paredes detecta la habitación",
            "Cliquez les coins et double-cliquez pour fermer · double-clic à l'intérieur des murs détecte la pièce",
            "Ecken klicken und Doppelklick schließt · Doppelklick in Wänden erkennt den Raum",
        ],
        "Clique os pontos e duplo clique encerra · na Elétrica/Hidráulica a linha vira eletroduto/tubulação" => [
            "Click the points and double-click to end · in Electrical/Plumbing lines become conduits/pipes",
            "Haga clic en los puntos y doble clic termina · en Eléctrica/Fontanería la línea se vuelve tubo/canalización",
            "Cliquez les points et double-cliquez pour terminer · en Électricité/Plomberie la ligne devient conduit/tuyau",
            "Punkte klicken und Doppelklick beendet · in Elektro/Sanitär werden Linien zu Leitungen/Rohren",
        ],
        "Clique para posicionar · portas e janelas encaixam na parede mais próxima · Esc cancela" => [
            "Click to place · doors and windows snap into the nearest wall · Esc cancels",
            "Haga clic para posicionar · puertas y ventanas encajan en la pared más cercana · Esc cancela",
            "Cliquez pour placer · portes et fenêtres s'encastrent dans le mur le plus proche · Échap annule",
            "Klicken zum Platzieren · Türen und Fenster rasten in der nächsten Wand ein · Esc bricht ab",
        ],
        "Clique seleciona · Ctrl+clique soma · arraste move · alças editam · duplo clique modifica" => [
            "Click selects · Ctrl+click adds · drag moves · handles edit · double-click modifies",
            "El clic selecciona · Ctrl+clic suma · arrastrar mueve · los tiradores editan · doble clic modifica",
            "Le clic sélectionne · Ctrl+clic ajoute · glisser déplace · les poignées modifient · double-clic édite",
            "Klick wählt · Strg+Klick fügt hinzu · Ziehen verschiebt · Griffe ändern · Doppelklick öffnet",
        ],
        "Colar" => ["Paste", "Pegar", "Coller", "Einfügen"],
        "Comece desenhando paredes (W), importe uma planta como imagem de fundo, ou peça para a IA via MCP." => [
            "Start by drawing walls (W), import a plan as background image, or ask the AI through MCP.",
            "Empiece dibujando paredes (W), importe un plano como imagen de fondo o pídalo a la IA vía MCP.",
            "Commencez par tracer des murs (W), importez un plan en image de fond, ou demandez-le à l'IA via MCP.",
            "Zuerst Wände zeichnen (W), einen Plan als Hintergrundbild importieren oder die KI über MCP fragen.",
        ],
        "Comparar" => ["Compare", "Comparar", "Comparer", "Vergleichen"],
        "Comparar as versões" => [
            "Compare versions",
            "Comparar las versiones",
            "Comparer les versions",
            "Versionen vergleichen",
        ],
        "Comparar versões" => [
            "Compare versions",
            "Comparar versiones",
            "Comparer les versions",
            "Versionen vergleichen",
        ],
        "Comprimento" => ["Length", "Longitud", "Longueur", "Länge"],
        "Comprimento exato da parede sendo desenhada" => [
            "Exact length of the wall being drawn",
            "Longitud exacta de la pared que se está dibujando",
            "Longueur exacte du mur en cours de tracé",
            "Genaue Länge der gezeichneten Wand",
        ],
        "Contorno" => ["Outline", "Contorno", "Contour", "Umriss"],
        "Contínuo" => ["Solid", "Continuo", "Continu", "Voll"],
        "Copiar" => ["Copy", "Copiar", "Copier", "Kopieren"],
        "Copiar · Recortar · Colar · Duplicar" => [
            "Copy · Cut · Paste · Duplicate",
            "Copiar · Cortar · Pegar · Duplicar",
            "Copier · Couper · Coller · Dupliquer",
            "Kopieren · Ausschneiden · Einfügen · Duplizieren",
        ],
        "Cor" => ["Color", "Color", "Couleur", "Farbe"],
        "Cotar paredes selecionadas" => [
            "Dimension selected walls",
            "Acotar paredes seleccionadas",
            "Coter les murs sélectionnés",
            "Gewählte Wände bemaßen",
        ],
        "Cotas automáticas (engenharia)" => [
            "Automatic dimensions (engineering)",
            "Cotas automáticas (ingeniería)",
            "Cotes automatiques (ingénierie)",
            "Automatische Maße (Technik)",
        ],
        "Criar cotas" => ["Create dimensions", "Crear cotas", "Créer des cotes", "Maße anlegen"],
        "Criar cômodos" => ["Create rooms", "Crear habitaciones", "Créer des pièces", "Räume anlegen"],
        "Criar foto" => ["Create photo", "Crear foto", "Créer une photo", "Foto erzeugen"],
        "Criar foto…" => ["Create photo…", "Crear foto…", "Créer une photo…", "Foto erzeugen…"],
        "Criar vídeo" => ["Create video", "Crear vídeo", "Créer une vidéo", "Video erzeugen"],
        "Criar vídeo…" => ["Create video…", "Crear vídeo…", "Créer une vidéo…", "Video erzeugen…"],
        "Pontos do caminho" => ["Path points", "Puntos del recorrido", "Points du parcours", "Pfadpunkte"],
        "Adicionar ponto atual" => [
            "Add current point",
            "Añadir punto actual",
            "Ajouter le point actuel",
            "Aktuellen Punkt hinzufügen",
        ],
        "Ative o visitante na vista 3D" => [
            "Turn on the visitor in the 3D view",
            "Active el visitante en la vista 3D",
            "Activez le visiteur dans la vue 3D",
            "Besucher in der 3D-Ansicht einschalten",
        ],
        "Órbita aérea" => ["Aerial orbit", "Órbita aérea", "Orbite aérienne", "Orbit von oben"],
        "Limpar" => ["Clear", "Limpiar", "Effacer", "Leeren"],
        "Quadros por segundo" => [
            "Frames per second",
            "Cuadros por segundo",
            "Images par seconde",
            "Bilder pro Sekunde",
        ],
        "Velocidade" => ["Speed", "Velocidad", "Vitesse", "Geschwindigkeit"],
        "Gerar vídeo…" => ["Render video…", "Generar vídeo…", "Générer la vidéo…", "Video rendern…"],
        "Vídeo salvo em" => [
            "Video saved to",
            "Vídeo guardado en",
            "Vidéo enregistrée dans",
            "Video gespeichert unter",
        ],
        "quadros" => ["frames", "cuadros", "images", "Bilder"],
        "Sol pela bússola e hora" => [
            "Sun from compass and time",
            "Sol según brújula y hora",
            "Soleil selon la boussole et l'heure",
            "Sonne aus Kompass und Uhrzeit",
        ],
        "Vista 3D" => ["3D view", "Vista 3D", "Vue 3D", "3D-Ansicht"],
        "Mostrar em 3D" => ["Show in 3D", "Mostrar en 3D", "Afficher en 3D", "In 3D zeigen"],
        "Inclinação" => ["Tilt", "Inclinación", "Inclinaison", "Neigung"],
        "Deitado" => ["Lying", "Tumbado", "Couché", "Liegend"],
        "Em pé" => ["Standing", "De pie", "Debout", "Stehend"],
        "Editor no navegador" => [
            "Browser editor",
            "Editor en el navegador",
            "Éditeur dans le navigateur",
            "Browser-Editor",
        ],
        "Disponível no aplicativo para desktop." => [
            "Available in the desktop app.",
            "Disponible en la aplicación de escritorio.",
            "Disponible dans l'application de bureau.",
            "In der Desktop-App verfügbar.",
        ],
        "No navegador, exporte em .glb." => [
            "In the browser, export as .glb.",
            "En el navegador, exporte en .glb.",
            "Dans le navigateur, exportez en .glb.",
            "Im Browser als .glb exportieren.",
        ],
        "Nenhum plugin instalado" => [
            "No plugins installed",
            "Ningún plugin instalado",
            "Aucun plugin installé",
            "Keine Plugins installiert",
        ],
        "Pastas" => ["Folders", "Carpetas", "Dossiers", "Ordner"],
        "Rodando" => ["Running", "En ejecución", "En cours", "Läuft"],
        "concluído" => ["done", "terminado", "terminé", "fertig"],
        "Acompanha as paredes" => ["Follows the walls", "Sigue las paredes", "Suit les murs", "Folgt den Wänden"],
        "O contorno é detectado pelas paredes e divisores e se ajusta quando eles mudam" => [
            "The outline is detected from walls and dividers and adjusts when they change",
            "El contorno se detecta por las paredes y los divisores y se ajusta cuando cambian",
            "Le contour est détecté d'après les murs et les séparations et s'ajuste quand ils changent",
            "Der Umriss kommt aus Wänden und Trennern und passt sich an, wenn sie sich ändern",
        ],
        "Ambientes" => ["Rooms", "Ambientes", "Espaces", "Räume"],
        "Divisor de ambiente" => ["Room divider", "Divisor de ambiente", "Séparation d'espace", "Raumtrenner"],
        "Separa ambientes sem parede (ex.: sala e jantar integrados)" => [
            "Separates rooms without a wall (e.g. open living and dining)",
            "Separa ambientes sin pared (p. ej.: salón y comedor integrados)",
            "Sépare des espaces sans mur (ex. : salon et salle à manger ouverts)",
            "Trennt Räume ohne Wand (z. B. offenes Wohnen und Essen)",
        ],
        "Escala vertical" => ["Vertical scale", "Escala vertical", "Échelle verticale", "Vertikaler Maßstab"],
        "Diferente" => ["Different", "Diferente", "Différent", "Anders"],
        "Criar paredes" => ["Create walls", "Crear paredes", "Créer des murs", "Wände anlegen"],
        "Curva suave" => ["Smooth curve", "Curva suave", "Courbe douce", "Weiche Kurve"],
        "Cômodos" => ["Rooms", "Habitaciones", "Pièces", "Räume"],
        "Da sua biblioteca de modelos. Clique para pôr no centro da planta" => [
            "From your model library. Click to put it in the middle of the plan",
            "De tu biblioteca de modelos. Haz clic para ponerlo en el centro del plano",
            "De votre bibliothèque de modèles. Cliquez pour le poser au centre du plan",
            "Aus Ihrer Modellbibliothek. Klicken, um es in die Mitte des Grundrisses zu setzen",
        ],
        "Descartar" => ["Discard", "Descartar", "Abandonner", "Verwerfen"],
        "Descrição" => ["Description", "Descripción", "Description", "Beschreibung"],
        "Desenhar linhas" => ["Draw lines", "Dibujar líneas", "Tracer des lignes", "Linien zeichnen"],
        "Desfazer" => ["Undo", "Deshacer", "Annuler", "Rückgängig"],
        "Desfazer (Ctrl+Z)" => ["Undo (Ctrl+Z)", "Deshacer (Ctrl+Z)", "Annuler (Ctrl+Z)", "Rückgängig (Strg+Z)"],
        "Desfazer · Refazer (inclusive o que a IA fez)" => [
            "Undo · Redo (including what the AI did)",
            "Deshacer · Rehacer (incluso lo que hizo la IA)",
            "Annuler · Rétablir (y compris ce qu'a fait l'IA)",
            "Rückgängig · Wiederholen (auch was die KI getan hat)",
        ],
        "Desliga o ímã (ângulos de 15°, pontos e grade)" => [
            "Disables magnetism (15° angles, points and grid)",
            "Desactiva el imán (ángulos de 15°, puntos y cuadrícula)",
            "Désactive l'aimant (angles de 15°, points et grille)",
            "Schaltet den Magneten aus (15°-Winkel, Punkte und Raster)",
        ],
        "Detalhes: marca, modelo e link" => [
            "Details: brand, model and link",
            "Detalles: marca, modelo y enlace",
            "Détails : marque, modèle et lien",
            "Details: Marke, Modell und Link",
        ],
        "Digitar número + Enter" => [
            "Type number + Enter",
            "Escribir número + Enter",
            "Taper un nombre + Entrée",
            "Zahl tippen + Enter",
        ],
        "Direita" => ["Right", "Derecha", "Droite", "Rechts"],
        "Disco" => ["Disc", "Disco", "Disque", "Scheibe"],
        "Distância real" => ["Real distance", "Distancia real", "Distance réelle", "Echte Distanz"],
        "Dividir parede ao meio" => [
            "Split wall in half",
            "Dividir la pared por la mitad",
            "Couper le mur en deux",
            "Wand halbieren",
        ],
        "Diâmetro" => ["Diameter", "Diámetro", "Diamètre", "Durchmesser"],
        "Dobradiça à direita" => [
            "Hinge on the right",
            "Bisagra a la derecha",
            "Charnière à droite",
            "Anschlag rechts",
        ],
        "Duplicar" => ["Duplicate", "Duplicar", "Dupliquer", "Duplizieren"],
        "Duplicar versão atual" => [
            "Duplicate current version",
            "Duplicar versión actual",
            "Dupliquer la version actuelle",
            "Aktuelle Version duplizieren",
        ],
        "Duplicar versão · Próxima versão (guias)" => [
            "Duplicate version · Next version (tabs)",
            "Duplicar versión · Siguiente versión (pestañas)",
            "Dupliquer la version · Version suivante (onglets)",
            "Version duplizieren · Nächste Version (Reiter)",
        ],
        "Duplo clique" => ["Double-click", "Doble clic", "Double-clic", "Doppelklick"],
        "Editar" => ["Edit", "Editar", "Modifier", "Bearbeiten"],
        "Editar andar…" => ["Edit storey…", "Editar planta…", "Modifier l'étage…", "Geschoss bearbeiten…"],
        "Elevação" => ["Elevation", "Elevación", "Élévation", "Höhe über Fußboden"],
        "Elevação do piso" => ["Floor elevation", "Elevación del suelo", "Niveau du sol", "Fußbodenhöhe"],
        "Elétrica" => ["Electrical", "Eléctrica", "Électricité", "Elektro"],
        "Elétrica (tomadas, luz, cabos)" => [
            "Electrical (outlets, lights, cables)",
            "Eléctrica (enchufes, luz, cables)",
            "Électricité (prises, éclairage, câbles)",
            "Elektro (Steckdosen, Leuchten, Kabel)",
        ],
        "Encerra paredes · fecha/detecta cômodo · cota parede · modifica" => [
            "Ends walls · closes/detects room · dimensions wall · modifies",
            "Termina paredes · cierra/detecta habitación · acota pared · modifica",
            "Termine les murs · ferme/détecte la pièce · cote le mur · modifie",
            "Beendet Wände · schließt/erkennt Raum · bemaßt Wand · ändert",
        ],
        "Enquadrar (Ctrl+0)" => ["Frame (Ctrl+0)", "Encuadrar (Ctrl+0)", "Cadrer (Ctrl+0)", "Einpassen (Strg+0)"],
        "Enquadrar 3D" => ["Frame 3D", "Encuadrar 3D", "Cadrer la 3D", "3D einpassen"],
        "Enquadrar planta" => ["Frame plan", "Encuadrar plano", "Cadrer le plan", "Grundriss einpassen"],
        "Escala" => ["Scale", "Escala", "Échelle", "Maßstab"],
        "Escala calibrada. Arraste para posicionar a imagem ou troque de ferramenta." => [
            "Scale calibrated. Drag to place the image or switch tools.",
            "Escala calibrada. Arrastre para posicionar la imagen o cambie de herramienta.",
            "Échelle calibrée. Faites glisser pour placer l'image ou changez d'outil.",
            "Maßstab kalibriert. Ziehen, um das Bild zu legen, oder Werkzeug wechseln.",
        ],
        "Espelhado" => ["Mirrored", "Reflejado", "Miroir", "Gespiegelt"],
        "Espessura" => ["Thickness", "Espesor", "Épaisseur", "Dicke"],
        "Espessura da laje" => [
            "Slab thickness",
            "Espesor de la losa",
            "Épaisseur de la dalle",
            "Deckenstärke",
        ],
        "Esquerda" => ["Left", "Izquierda", "Gauche", "Links"],
        "Esta versão e o histórico dela serão removidos do projeto." => [
            "This version and its history will be removed from the project.",
            "Esta versión y su historial se quitarán del proyecto.",
            "Cette version et son historique seront retirés du projet.",
            "Diese Version und ihre Geschichte werden aus dem Projekt entfernt.",
        ],
        "Estilo" => ["Style", "Estilo", "Style", "Stil"],
        "Excluir" => ["Delete", "Eliminar", "Supprimer", "Löschen"],
        "Excluir andar" => ["Delete storey", "Eliminar planta", "Supprimer l'étage", "Geschoss löschen"],
        "Exibir" => ["Show", "Ver", "Afficher", "Anzeigen"],
        "Exportar 3D" => ["Export 3D", "Exportar 3D", "Exporter la 3D", "3D exportieren"],
        "Export walls" => ["Export walls", "Exportar paredes", "Exporter les murs", "Wände exportieren"],
        "Exportar paredes" => ["Export walls", "Exportar paredes", "Exporter les murs", "Wände exportieren"],
        "Exportar planta" => ["Export plan", "Exportar plano", "Exporter le plan", "Grundriss exportieren"],
        "CSV (spreadsheet)…" => [
            "CSV (spreadsheet)…",
            "CSV (hoja de cálculo)…",
            "CSV (tableur)…",
            "CSV (Tabelle)…",
        ],
        "CSV (planilha)…" => [
            "CSV (spreadsheet)…",
            "CSV (hoja de cálculo)…",
            "CSV (tableur)…",
            "CSV (Tabelle)…",
        ],
        "DXF (CAD)…" => ["DXF (CAD)…", "DXF (CAD)…", "DXF (CAO)…", "DXF (CAD)…"],
        "JSON…" => ["JSON…", "JSON…", "JSON…", "JSON…"],
        "Fechada" => ["Closed", "Cerrada", "Fermée", "Geschlossen"],
        "Fechar" => ["Close", "Cerrar", "Fermer", "Schließen"],
        "Fechar versão" => ["Close version", "Cerrar versión", "Fermer la version", "Version schließen"],
        "Fim" => ["End", "Fin", "Fin", "Ende"],
        "Fim (x, y)" => ["End (x, y)", "Fin (x, y)", "Fin (x, y)", "Ende (x, y)"],
        "Grupo" => ["Group", "Grupo", "Groupe", "Gruppe"],
        "Hidráulica" => ["Plumbing", "Fontanería", "Plomberie", "Sanitär"],
        "Hora do dia" => ["Time of day", "Hora del día", "Heure de la journée", "Tageszeit"],
        "Imagem" => ["Image", "Imagen", "Image", "Bild"],
        "Imagem de fundo" => ["Background image", "Imagen de fondo", "Image de fond", "Hintergrundbild"],
        "Imagem importada. Clique em dois pontos de medida conhecida para calibrar; arraste para posicionar." => [
            "Image imported. Click two points of a known distance to calibrate; drag to place.",
            "Imagen importada. Haga clic en dos puntos de distancia conocida para calibrar; arrastre para posicionar.",
            "Image importée. Cliquez deux points d'une distance connue pour calibrer ; faites glisser pour placer.",
            "Bild importiert. Zwei Punkte mit bekanntem Abstand klicken zum Kalibrieren; ziehen zum Legen.",
        ],
        "Imagem…" => ["Image…", "Imagen…", "Image…", "Bild…"],
        "Imagens" => ["Images", "Imágenes", "Images", "Bilder"],
        "Importar imagem de fundo" => [
            "Import background image",
            "Importar imagen de fondo",
            "Importer une image de fond",
            "Hintergrundbild importieren",
        ],
        "Importar modelo 3D…" => [
            "Import 3D model…",
            "Importar modelo 3D…",
            "Importer un modèle 3D…",
            "3D-Modell importieren…",
        ],
        "Importar…" => ["Import…", "Importar…", "Importer…", "Importieren…"],
        "Informações" => ["Information", "Información", "Informations", "Informationen"],
        "Início" => ["Start", "Inicio", "Début", "Anfang"],
        "Início (x, y)" => ["Start (x, y)", "Inicio (x, y)", "Début (x, y)", "Anfang (x, y)"],
        "Itálico" => ["Italic", "Cursiva", "Italique", "Kursiv"],
        "Lado direito" => ["Right side", "Lado derecho", "Côté droit", "Rechte Seite"],
        "Lado esquerdo" => ["Left side", "Lado izquierdo", "Côté gauche", "Linke Seite"],
        "Largura" => ["Width", "Anchura", "Largeur", "Breite"],
        "Legenda de símbolos (elétrica e hidráulica)" => [
            "Symbol legend (electrical and plumbing)",
            "Leyenda de símbolos (eléctrica y fontanería)",
            "Légende des symboles (électricité et plomberie)",
            "Symbollegende (Elektro und Sanitär)",
        ],
        "Licença" => ["License", "Licencia", "Licence", "Lizenz"],
        "Linhas (tubulação / eletroduto)" => [
            "Lines (pipes / conduits)",
            "Líneas (tubería / tubo eléctrico)",
            "Lignes (tuyaux / conduits)",
            "Linien (Rohre / Leitungen)",
        ],
        "Link" => ["Link", "Enlace", "Lien", "Link"],
        "MCP desligado" => ["MCP off", "MCP apagado", "MCP désactivé", "MCP aus"],
        "Manter proporções" => [
            "Keep proportions",
            "Mantener proporciones",
            "Conserver les proportions",
            "Proportionen halten",
        ],
        "Marca" => ["Brand", "Marca", "Marque", "Marke"],
        "Me paga um café" => ["Buy me a coffee", "Invítame un café", "Offre-moi un café", "Buy me a coffee"],
        "Medida" => ["Measure", "Medida", "Mesure", "Messen"],
        "Minha biblioteca" => ["My library", "Mi biblioteca", "Ma bibliothèque", "Meine Bibliothek"],
        "Modelo" => ["Model", "Modelo", "Modèle", "Modell"],
        "Modelo da biblioteca posicionado. Arraste para o lugar." => [
            "Library model placed. Drag it into place.",
            "Modelo de la biblioteca colocado. Arrástralo a su lugar.",
            "Modèle de la bibliothèque posé. Faites-le glisser à sa place.",
            "Bibliotheksmodell gesetzt. An den Ort ziehen.",
        ],
        "Modelo importado. Ajuste medidas com Enter ou pelas alças." => [
            "Model imported. Adjust sizes with Enter or the handles.",
            "Modelo importado. Ajuste las medidas con Enter o con los tiradores.",
            "Modèle importé. Ajustez les dimensions avec Entrée ou les poignées.",
            "Modell importiert. Größen mit Enter oder den Griffen anpassen.",
        ],
        "Modelos 3D" => ["3D models", "Modelos 3D", "Modèles 3D", "3D-Modelle"],
        "Modificar andar" => ["Modify storey", "Modificar planta", "Modifier l'étage", "Geschoss ändern"],
        "Modificar cota" => ["Modify dimension", "Modificar cota", "Modifier la cote", "Maß ändern"],
        "Modificar cômodo" => ["Modify room", "Modificar habitación", "Modifier la pièce", "Raum ändern"],
        "Modificar linha" => ["Modify line", "Modificar línea", "Modifier la ligne", "Linie ändern"],
        "Modificar parede" => ["Modify wall", "Modificar pared", "Modifier le mur", "Wand ändern"],
        "Modificar texto" => ["Modify text", "Modificar texto", "Modifier le texte", "Text ändern"],
        "Modificar · Excluir · Cancelar" => [
            "Modify · Delete · Cancel",
            "Modificar · Eliminar · Cancelar",
            "Modifier · Supprimer · Annuler",
            "Ändern · Löschen · Abbrechen",
        ],
        "Modificar…" => ["Modify…", "Modificar…", "Modifier…", "Ändern…"],
        "Mostrar bússola" => ["Show compass", "Mostrar brújula", "Afficher la boussole", "Kompass zeigen"],
        "Move a seleção 1 cm (10 cm)" => [
            "Moves the selection 1 cm (10 cm)",
            "Mueve la selección 1 cm (10 cm)",
            "Déplace la sélection de 1 cm (10 cm)",
            "Verschiebt die Auswahl um 1 cm (10 cm)",
        ],
        "Mover vista" => ["Pan view", "Mover vista", "Déplacer la vue", "Ansicht verschieben"],
        "Máxima" => ["Best", "Máxima", "Maximale", "Beste"],
        "Móveis" => ["Furniture", "Muebles", "Mobilier", "Möbel"],
        "Móveis na planta" => [
            "Furniture on the plan",
            "Muebles en el plano",
            "Mobilier sur le plan",
            "Möbel im Grundriss",
        ],
        "Nada encontrado." => ["Nothing found.", "No se encontró nada.", "Aucun résultat.", "Nichts gefunden."],
        "Negrito" => ["Bold", "Negrita", "Gras", "Fett"],
        "Nenhum espaço fechado por paredes aqui" => [
            "No space enclosed by walls here",
            "Ningún espacio cerrado por paredes aquí",
            "Aucun espace fermé par des murs ici",
            "Hier umschließen keine Wände einen Raum",
        ],
        "Nenhum ponto" => ["No points", "Ningún punto", "Aucun point", "Keine Punkte"],
        "Nenhum ponto de vista salvo" => [
            "No saved points of view",
            "Ningún punto de vista guardado",
            "Aucun point de vue enregistré",
            "Keine gespeicherten Blickpunkte",
        ],
        "Nome" => ["Name", "Nombre", "Nom", "Name"],
        "Nome do projeto" => ["Project name", "Nombre del proyecto", "Nom du projet", "Projektname"],
        "Norte" => ["North", "Norte", "Nord", "Norden"],
        "Nova versão da planta" => [
            "New plan version",
            "Nueva versión del plano",
            "Nouvelle version du plan",
            "Neue Planversion",
        ],
        "Nova versão em branco" => [
            "New blank version",
            "Nueva versión en blanco",
            "Nouvelle version vierge",
            "Neue leere Version",
        ],
        "Novo" => ["New", "Nuevo", "Nouveau", "Neu"],
        "Novo (Ctrl+N)" => ["New (Ctrl+N)", "Nuevo (Ctrl+N)", "Nouveau (Ctrl+N)", "Neu (Strg+N)"],
        "Níveis na mesma elevação funcionam como layouts alternativos" => [
            "Levels at the same elevation work as alternative layouts",
            "Los niveles a la misma elevación funcionan como diseños alternativos",
            "Des niveaux à la même élévation servent de variantes d'aménagement",
            "Geschosse auf gleicher Höhe wirken als alternative Layouts",
        ],
        "O andar e tudo o que está nele serão removidos (dá para desfazer)." => [
            "The storey and everything on it will be removed (undoable).",
            "La planta y todo lo que hay en ella se quitarán (se puede deshacer).",
            "L'étage et tout ce qu'il contient seront supprimés (annulable).",
            "Das Geschoss und alles darauf wird entfernt (rückgängig machbar).",
        ],
        "O projeto tem alterações que ainda não foram salvas." => [
            "The project has unsaved changes.",
            "El proyecto tiene cambios sin guardar.",
            "Le projet a des modifications non enregistrées.",
            "Das Projekt hat ungespeicherte Änderungen.",
        ],
        "OBJ + MTL…" => ["OBJ + MTL…", "OBJ + MTL…", "OBJ + MTL…", "OBJ + MTL…"],
        "OK" => ["OK", "OK", "OK", "OK"],
        "Opacidade" => ["Opacity", "Opacidad", "Opacité", "Deckkraft"],
        "Ordem (mesma elevação)" => [
            "Order (same elevation)",
            "Orden (misma elevación)",
            "Ordre (même élévation)",
            "Reihenfolge (gleiche Höhe)",
        ],
        "PDF (A3, fit to sheet)…" => [
            "PDF (A3, fit to sheet)…",
            "PDF (A3, ajustado a la hoja)…",
            "PDF (A3, ajusté à la feuille)…",
            "PDF (A3, Blatt füllen)…",
        ],
        "PDF (A3, ajustado à folha)…" => [
            "PDF (A3, fit to sheet)…",
            "PDF (A3, ajustado a la hoja)…",
            "PDF (A3, ajusté à la feuille)…",
            "PDF (A3, Blatt füllen)…",
        ],
        "PDF (A4, one wall per page)…" => [
            "PDF (A4, one wall per page)…",
            "PDF (A4, una pared por hoja)…",
            "PDF (A4, un mur par page)…",
            "PDF (A4, eine Wand pro Seite)…",
        ],
        "PDF (A4, uma parede por folha)…" => [
            "PDF (A4, one wall per page)…",
            "PDF (A4, una pared por hoja)…",
            "PDF (A4, un mur par page)…",
            "PDF (A4, eine Wand pro Seite)…",
        ],
        "PDF 1:100…" => ["PDF 1:100…", "PDF 1:100…", "PDF 1:100…", "PDF 1:100…"],
        "PDF 1:50…" => ["PDF 1:50…", "PDF 1:50…", "PDF 1:50…", "PDF 1:50…"],
        "PNG…" => ["PNG…", "PNG…", "PNG…", "PNG…"],
        "Paredes" => ["Walls", "Paredes", "Murs", "Wände"],
        "Inteiras" => ["Up", "Enteras", "Debout", "Hoch"],
        "Recortadas" => ["Cutaway", "Recortadas", "Coupés", "Schnitt"],
        "Baixas" => ["Down", "Bajas", "Baissés", "Runter"],
        "Todas as paredes em pé" => [
            "Every wall standing",
            "Todas las paredes en pie",
            "Tous les murs debout",
            "Alle Wände stehen",
        ],
        "As paredes da frente descem para mostrar os cômodos" => [
            "The walls in front come down to show the rooms",
            "Las paredes de delante bajan para mostrar las habitaciones",
            "Les murs de devant s'abaissent pour montrer les pièces",
            "Die Wände vorn klappen herunter, damit die Räume sichtbar sind",
        ],
        "Todas as paredes descem" => [
            "Every wall comes down",
            "Todas las paredes bajan",
            "Tous les murs s'abaissent",
            "Alle Wände klappen herunter",
        ],
        "Personalizada" => ["Custom", "Personalizada", "Personnalisée", "Eigene"],
        "Peça" => ["Tile", "Azulejo", "Carrelage", "Fliese"],
        "Pintura" => ["Paint", "Pintura", "Peinture", "Farbe"],
        "Piso" => ["Floor", "Suelo", "Sol", "Boden"],
        "Planta" => ["Plan", "Plano", "Plan", "Grundriss"],
        "Pontilhado" => ["Dotted", "Punteado", "Pointillé", "Gepunktet"],
        "Posição (x, y)" => ["Position (x, y)", "Posición (x, y)", "Position (x, y)", "Position (x, y)"],
        "Potência da luz" => [
            "Light power",
            "Potencia de la luz",
            "Puissance de la lumière",
            "Lichtleistung",
        ],
        "Preço" => ["Price", "Precio", "Prix", "Preis"],
        "Problemas" => ["Issues", "Problemas", "Problèmes", "Hinweise"],
        "Profundidade" => ["Depth", "Profundidad", "Profondeur", "Tiefe"],
        "Projeto em edição: novos símbolos e linhas vão para ele" => [
            "Project being edited: new symbols and lines go there",
            "Proyecto en edición: los nuevos símbolos y líneas van a él",
            "Projet en cours d'édition : les nouveaux symboles et lignes y vont",
            "Bearbeitetes Projekt: neue Symbole und Linien landen dort",
        ],
        "Projetos (3D New Era AI, Sweet Home 3D)" => [
            "Projects (3D New Era AI, Sweet Home 3D)",
            "Proyectos (3D New Era AI, Sweet Home 3D)",
            "Projets (3D New Era AI, Sweet Home 3D)",
            "Projekte (3D New Era AI, Sweet Home 3D)",
        ],
        "Pé-direito" => ["Ceiling height", "Altura libre", "Hauteur sous plafond", "Raumhöhe"],
        "Qualidade" => ["Quality", "Calidad", "Qualité", "Qualität"],
        "Quantitativos" => ["Quantities", "Mediciones", "Quantitatifs", "Mengen"],
        "Rascunho" => ["Draft", "Borrador", "Brouillon", "Entwurf"],
        "Recentes" => ["Recent", "Recientes", "Récents", "Zuletzt"],
        "Recortar" => ["Cut", "Cortar", "Couper", "Ausschneiden"],
        "Refazer" => ["Redo", "Rehacer", "Rétablir", "Wiederholen"],
        "Refazer (Ctrl+Shift+Z)" => [
            "Redo (Ctrl+Shift+Z)",
            "Rehacer (Ctrl+Shift+Z)",
            "Rétablir (Ctrl+Maj+Z)",
            "Wiederholen (Strg+Umschalt+Z)",
        ],
        "Referências dos cômodos" => [
            "Room references",
            "Referencias de las habitaciones",
            "Références des pièces",
            "Raumverzeichnis",
        ],
        "Remover imagem" => ["Remove image", "Quitar imagen", "Retirer l'image", "Bild entfernen"],
        "Renderizar" => ["Render", "Renderizar", "Rendu", "Rendern"],
        "Renomear" => ["Rename", "Renombrar", "Renommer", "Umbenennen"],
        "Rotação" => ["Rotation", "Rotación", "Rotation", "Drehung"],
        "SVG em escala real…" => [
            "SVG at true scale…",
            "SVG a escala real…",
            "SVG à l'échelle réelle…",
            "SVG im echten Maßstab…",
        ],
        "Sair" => ["Quit", "Salir", "Quitter", "Beenden"],
        "Salvar" => ["Save", "Guardar", "Enregistrer", "Speichern"],
        "Salvar (Ctrl+S)" => ["Save (Ctrl+S)", "Guardar (Ctrl+S)", "Enregistrer (Ctrl+S)", "Speichern (Strg+S)"],
        "Salvar PNG…" => ["Save PNG…", "Guardar PNG…", "Enregistrer en PNG…", "PNG speichern…"],
        "Salvar alterações?" => [
            "Save changes?",
            "¿Guardar los cambios?",
            "Enregistrer les modifications ?",
            "Änderungen speichern?",
        ],
        "Salvar como…" => ["Save as…", "Guardar como…", "Enregistrer sous…", "Speichern unter…"],
        "Salvar ponto de vista" => [
            "Save point of view",
            "Guardar punto de vista",
            "Enregistrer le point de vue",
            "Blickpunkt speichern",
        ],
        "Scroll · botão do meio · F" => [
            "Scroll · middle button · F",
            "Rueda · botón central · F",
            "Molette · bouton du milieu · F",
            "Scrollen · Mitteltaste · F",
        ],
        "Selecionar" => ["Select", "Seleccionar", "Sélectionner", "Auswählen"],
        "Selecionar tudo" => ["Select all", "Seleccionar todo", "Tout sélectionner", "Alles auswählen"],
        "Selecionar · Mover vista · Paredes · Cômodos · Cotas · Texto" => [
            "Select · Pan · Walls · Rooms · Dimensions · Text",
            "Seleccionar · Mover vista · Paredes · Habitaciones · Cotas · Texto",
            "Sélectionner · Déplacer · Murs · Pièces · Cotes · Texte",
            "Auswählen · Verschieben · Wände · Räume · Maße · Text",
        ],
        "Sem acabamento" => ["No finish", "Sin acabado", "Sans finition", "Ohne Belag"],
        "Sem nome" => ["Unnamed", "Sin nombre", "Sans nom", "Ohne Namen"],
        "Sem seta" => ["No arrow", "Sin flecha", "Sans flèche", "Kein Pfeil"],
        "Seta aberta" => ["Open arrow", "Flecha abierta", "Flèche ouverte", "Offener Pfeil"],
        "Seta cheia" => ["Filled arrow", "Flecha rellena", "Flèche pleine", "Gefüllter Pfeil"],
        "Setas (+Shift)" => ["Arrows (+Shift)", "Flechas (+Mayús)", "Flèches (+Maj)", "Pfeile (+Umschalt)"],
        "Shift (segurado)" => ["Shift (held)", "Mayús (mantenido)", "Maj (maintenue)", "Umschalt (gehalten)"],
        "Sweet Home 3D" => ["Sweet Home 3D", "Sweet Home 3D", "Sweet Home 3D", "Sweet Home 3D"],
        "Símbolos arquitetônicos" => [
            "Architectural symbols",
            "Símbolos arquitectónicos",
            "Symboles architecturaux",
            "Architektursymbole",
        ],
        "Tamanho" => ["Size", "Tamaño", "Taille", "Größe"],
        "Teto" => ["Ceiling", "Techo", "Plafond", "Decke"],
        "Texto" => ["Text", "Texto", "Texte", "Text"],
        "Tipo" => ["Type", "Tipo", "Type", "Typ"],
        "Tracejado" => ["Dashed", "Discontinuo", "Tirets", "Gestrichelt"],
        "Traço" => ["Dash", "Raya", "Tiret", "Strich"],
        "Traço e dois pontos" => ["Dash dot dot", "Raya y dos puntos", "Tiret point point", "Strich Punkt Punkt"],
        "Traço e ponto" => ["Dash dot", "Raya y punto", "Tiret point", "Strich Punkt"],
        "Térreo" => ["Ground floor", "Planta baja", "Rez-de-chaussée", "Erdgeschoss"],
        "{}º andar" => ["Floor {}", "Planta {}", "Étage {}", "{}. Obergeschoss"],
        "Unidade" => ["Unit", "Unidad", "Unité", "Einheit"],
        "Unir paredes selecionadas" => [
            "Join selected walls",
            "Unir paredes seleccionadas",
            "Joindre les murs sélectionnés",
            "Gewählte Wände verbinden",
        ],
        "Usa o ponto de vista atual da vista 3D (aérea ou visitante)." => [
            "Uses the current 3D point of view (aerial or visitor).",
            "Usa el punto de vista actual de la vista 3D (aérea o visitante).",
            "Utilise le point de vue actuel de la vue 3D (aérienne ou visiteur).",
            "Nutzt den aktuellen 3D-Blickpunkt (Orbit oder Besucher).",
        ],
        "Ver" => ["View", "Ver", "Affichage", "Ansicht"],
        "Versão" => ["Version", "Versión", "Version", "Version"],
        "Visitante" => ["Visitor", "Visitante", "Visiteur", "Besucher"],
        "Visitante — arraste: olhar · W/A/S/D ou setas: andar · Scroll: avançar · Esc: visão aérea" => [
            "Visitor — drag: look · W/A/S/D or arrows: walk · Scroll: forward · Esc: aerial view",
            "Visitante — arrastrar: mirar · W/A/S/D o flechas: andar · Rueda: avanzar · Esc: vista aérea",
            "Visiteur — glisser : regarder · W/A/S/D ou flèches : marcher · Molette : avancer · Échap : vue aérienne",
            "Besucher — ziehen: umsehen · W/A/S/D oder Pfeile: gehen · Scrollen: vor · Esc: Vogelperspektive",
        ],
        "Vista de cima de todos" => [
            "Top views for all",
            "Vista superior de todos",
            "Vues de dessus pour tout",
            "Draufsichten für alle",
        ],
        "Visualização 3D requer o backend wgpu." => [
            "The 3D view needs the wgpu backend.",
            "La vista 3D necesita el backend wgpu.",
            "La vue 3D nécessite le backend wgpu.",
            "Die 3D-Ansicht braucht das wgpu-Backend.",
        ],
        "Visão aérea" => ["Aerial view", "Vista aérea", "Vue aérienne", "Vogelperspektive"],
        "Frente" => ["Front", "Frente", "Face", "Vorn"],
        "Trás" => ["Back", "Detrás", "Arrière", "Hinten"],
        "Topo" => ["Top", "Superior", "Dessus", "Oben"],
        "Visível" => ["Visible", "Visible", "Visible", "Sichtbar"],
        "Visível no 3D" => ["Visible in 3D", "Visible en 3D", "Visible en 3D", "In 3D sichtbar"],
        "Zoom · mover vista · enquadrar" => [
            "Zoom · pan · frame",
            "Zoom · mover vista · encuadrar",
            "Zoom · déplacer · cadrer",
            "Zoom · verschieben · einpassen",
        ],
        "glTF binário (.glb)…" => [
            "Binary glTF (.glb)…",
            "glTF binario (.glb)…",
            "glTF binaire (.glb)…",
            "Binäres glTF (.glb)…",
        ],
        "peças" => ["pieces", "piezas", "pièces", "Stücke"],
        "Área" => ["Area", "Área", "Surface", "Fläche"],
        "Área na planta" => [
            "Area on the plan",
            "Área en el plano",
            "Surface sur le plan",
            "Fläche im Grundriss",
        ],
        "Ângulo" => ["Angle", "Ángulo", "Angle", "Winkel"],
        "Cotas" => ["Dimensions", "Cotas", "Cotes", "Maße"],
        "Textos" => ["Texts", "Textos", "Textes", "Texte"],
        "Linhas" => ["Lines", "Líneas", "Lignes", "Linien"],
        "paredes" => ["walls", "paredes", "murs", "Wände"],
        "cômodos" => ["rooms", "habitaciones", "pièces", "Räume"],
        "móveis" => ["pieces", "muebles", "meubles", "Stücke"],
        "grupo de" => ["group of", "grupo de", "groupe de", "Gruppe aus"],
        "Sala de estar" => ["Living room", "Salón", "Salon", "Wohnzimmer"],
        "Sala de jantar" => ["Dining room", "Comedor", "Salle à manger", "Esszimmer"],
        "Cozinha" => ["Kitchen", "Cocina", "Cuisine", "Küche"],
        "Quarto" => ["Bedroom", "Dormitorio", "Chambre", "Schlafzimmer"],
        "Banheiro" => ["Bathroom", "Baño", "Salle de bain", "Bad"],
        "Lavanderia" => ["Laundry", "Lavadero", "Buanderie", "Waschküche"],
        "Escritório" => ["Office", "Despacho", "Bureau", "Büro"],
        "Portas e janelas" => [
            "Doors and windows",
            "Puertas y ventanas",
            "Portes et fenêtres",
            "Türen und Fenster",
        ],
        "Estrutura" => ["Structure", "Estructura", "Structure", "Konstruktion"],
        "Decoração" => ["Decor", "Decoración", "Décoration", "Dekor"],
        "Área externa" => ["Outdoor", "Zona exterior", "Extérieur", "Außen"],
        "atual" => ["current", "actual", "actuel", "aktuell"],
        "Importado de {} — salve como projeto para manter tudo num arquivo" => [
            "Imported from {} — save it as a project to keep everything in one file",
            "Importado de {} — guárdelo como proyecto para tenerlo todo en un archivo",
            "Importé de {} — enregistrez-le en projet pour tout garder dans un seul fichier",
            "Importiert aus {} — als Projekt speichern, damit alles in einer Datei bleibt",
        ],
        "Aberto: {}" => ["Opened: {}", "Abierto: {}", "Ouvert : {}", "Geöffnet: {}"],
        " · {} aviso(s)" => [
            " · {} warning(s)",
            " · {} aviso(s)",
            " · {} avertissement(s)",
            " · {} Warnung(en)",
        ],
        "⚠ Não foi possível abrir {}: {}" => [
            "⚠ Could not open {}: {}",
            "⚠ No se pudo abrir {}: {}",
            "⚠ Impossible d'ouvrir {} : {}",
            "⚠ {} ließ sich nicht öffnen: {}",
        ],
        "Pontos de vista ({})" => [
            "Points of view ({})",
            "Puntos de vista ({})",
            "Points de vue ({})",
            "Blickpunkte ({})",
        ],
        "Ponto de vista {}" => ["Point of view {}", "Punto de vista {}", "Point de vue {}", "Blickpunkt {}"],
        "Salvo em {}" => ["Saved to {}", "Guardado en {}", "Enregistré dans {}", "Gespeichert unter {}"],
        "⚠ Não foi possível salvar: {}" => [
            "⚠ Could not save: {}",
            "⚠ No se pudo guardar: {}",
            "⚠ Impossible d'enregistrer : {}",
            "⚠ Speichern fehlgeschlagen: {}",
        ],
        "Modelo 3D exportado para {}" => [
            "3D model exported to {}",
            "Modelo 3D exportado a {}",
            "Modèle 3D exporté vers {}",
            "3D-Modell exportiert nach {}",
        ],
        "⚠ Falha ao exportar: {}" => [
            "⚠ Export failed: {}",
            "⚠ Fallo al exportar: {}",
            "⚠ Échec de l'export : {}",
            "⚠ Export fehlgeschlagen: {}",
        ],
        "Planta exportada para {}" => [
            "Plan exported to {}",
            "Plano exportado a {}",
            "Plan exporté vers {}",
            "Grundriss exportiert nach {}",
        ],
        "Paredes exportadas para {}" => [
            "Walls exported to {}",
            "Paredes exportadas a {}",
            "Murs exportés vers {}",
            "Wände exportiert nach {}",
        ],
        "Nenhuma parede para exportar." => [
            "No walls to export.",
            "No hay paredes para exportar.",
            "Aucun mur à exporter.",
            "Keine Wände zum Export.",
        ],
        "⚠ Imagem inválida: {}" => [
            "⚠ Invalid image: {}",
            "⚠ Imagen no válida: {}",
            "⚠ Image non valide : {}",
            "⚠ Ungültiges Bild: {}",
        ],
        "Modificar {} paredes" => [
            "Modify {} walls",
            "Modificar {} paredes",
            "Modifier {} murs",
            "{} Wände ändern",
        ],
        "Modificar {}" => ["Modify {}", "Modificar {}", "Modifier {}", "{} ändern"],
        "{} peças" => ["{} pieces", "{} piezas", "{} pièces", "{} Stücke"],
        "Os pontos marcados estão a {} na escala atual." => [
            "The marked points are {} apart at the current scale.",
            "Los puntos marcados están a {} en la escala actual.",
            "Les points marqués sont distants de {} à l'échelle actuelle.",
            "Die markierten Punkte liegen im aktuellen Maßstab {} auseinander.",
        ],
        "Fechar “{}”?" => ["Close “{}”?", "¿Cerrar «{}»?", "Fermer « {} » ?", "„{}“ schließen?"],
        "Excluir “{}”?" => ["Delete “{}”?", "¿Eliminar «{}»?", "Supprimer « {} » ?", "„{}“ löschen?"],
        "Renderizando… {} s" => ["Rendering… {} s", "Renderizando… {} s", "Rendu… {} s", "Rendern… {} s"],
        "Pronta em {} s" => ["Ready in {} s", "Lista en {} s", "Prête en {} s", "Fertig in {} s"],
        "Foto salva em {}" => [
            "Photo saved to {}",
            "Foto guardada en {}",
            "Photo enregistrée dans {}",
            "Foto gespeichert unter {}",
        ],
        "Mostrar {}" => ["Show {}", "Mostrar {}", "Afficher {}", "{} zeigen"],
        "Tema" => ["Theme", "Tema", "Thème", "Thema"],
        "Do sistema" => ["From the system", "Del sistema", "Du système", "Vom System"],
        "Noite" => ["Night", "Noche", "Nuit", "Nacht"],
        "Dia" => ["Day", "Día", "Jour", "Tag"],
        "luz" => ["light", "luz", "lumière", "hell"],
        "Arraste ou dois dedos: girar · Shift: mover · Pinça ou scroll: zoom · F: enquadrar" => [
            "Drag or two fingers: orbit · Shift: pan · Pinch or scroll: zoom · F: frame",
            "Arrastrar o dos dedos: girar · Mayús: mover · Pellizco o rueda: zoom · F: encuadrar",
            "Glisser ou deux doigts : pivoter · Maj : déplacer · Pincer ou molette : zoom · F : cadrer",
            "Ziehen oder zwei Finger: umkreisen · Umschalt: verschieben · Kneifen oder scrollen: zoomen · F: einpassen",
        ],
        "Aparência" => ["Look", "Aspecto", "Apparence", "Aussehen"],
        // Every piece of the catalog: what the window calls it, and what it
        // is named when someone drops it on the plan.
        "Sofá 3 lugares" => ["3-seat sofa", "Sofá de 3 plazas", "Canapé 3 places", "3-Sitzer-Sofa"],
        "Sofá 2 lugares" => ["2-seat sofa", "Sofá de 2 plazas", "Canapé 2 places", "2-Sitzer-Sofa"],
        "Sofá em L (chaise)" => [
            "L-shaped sofa (chaise)",
            "Sofá en L (chaise longue)",
            "Canapé d'angle (méridienne)",
            "L-Sofa (Recamiere)",
        ],
        "Poltrona" => ["Armchair", "Sillón", "Fauteuil", "Sessel"],
        "Poltrona Win" => ["Win armchair", "Sillón Win", "Fauteuil Win", "Win-Sessel"],
        "Cadeira Ares" => ["Ares chair", "Silla Ares", "Chaise Ares", "Ares-Stuhl"],
        "Mesa de centro" => ["Coffee table", "Mesa de centro", "Table basse", "Couchtisch"],
        "Mesa lateral" => ["Side table", "Mesa auxiliar", "Table d'appoint", "Beistelltisch"],
        "Rack de TV" => ["TV unit", "Mueble de TV", "Meuble TV", "TV-Möbel"],
        "Televisão 55\"" => ["55\" television", "Televisor de 55\"", "Téléviseur 55\"", "55\"-Fernseher"],
        "Estante" => ["Shelving unit", "Estantería", "Étagère", "Regal"],
        "Tapete" => ["Rug", "Alfombra", "Tapis", "Teppich"],
        "Radiador de painel 60×10×50" => [
            "Panel radiator 60×10×50",
            "Radiador de panel 60×10×50",
            "Radiateur à panneau 60×10×50",
            "Plattenheizkörper 60×10×50",
        ],
        "Radiador de painel 100×10×50" => [
            "Panel radiator 100×10×50",
            "Radiador de panel 100×10×50",
            "Radiateur à panneau 100×10×50",
            "Plattenheizkörper 100×10×50",
        ],
        "Radiador de painel 120×10×50" => [
            "Panel radiator 120×10×50",
            "Radiador de panel 120×10×50",
            "Radiateur à panneau 120×10×50",
            "Plattenheizkörper 120×10×50",
        ],
        "Convector elétrico 100×10×45" => [
            "Electric convector 100×10×45",
            "Convector eléctrico 100×10×45",
            "Convecteur électrique 100×10×45",
            "Elektrokonvektor 100×10×45",
        ],
        "Luminária de piso" => ["Floor lamp", "Lámpara de pie", "Lampadaire", "Stehleuchte"],
        "Abajur de mesa" => ["Table lamp", "Lámpara de mesa", "Lampe de table", "Tischleuchte"],
        "Spot embutido LED 7 W" => [
            "Recessed LED spot 7 W",
            "Foco LED empotrado 7 W",
            "Spot LED encastré 7 W",
            "LED-Einbauspot 7 W",
        ],
        "Pendente" => ["Pendant light", "Lámpara colgante", "Suspension", "Pendelleuchte"],
        "Painel LED 60×60 36 W" => [
            "LED panel 60×60 36 W",
            "Panel LED 60×60 36 W",
            "Dalle LED 60×60 36 W",
            "LED-Panel 60×60 36 W",
        ],
        "Fita LED (perfil)" => [
            "LED strip (profile)",
            "Tira LED (perfil)",
            "Ruban LED (profilé)",
            "LED-Streifen (Profil)",
        ],
        "Mesa de jantar 4 lugares" => [
            "Dining table for 4",
            "Mesa de comedor de 4",
            "Table à manger 4 places",
            "Esstisch für 4",
        ],
        "Mesa de jantar 6 lugares" => [
            "Dining table for 6",
            "Mesa de comedor de 6",
            "Table à manger 6 places",
            "Esstisch für 6",
        ],
        "Mesa com 4 cadeiras" => [
            "Table with 4 chairs",
            "Mesa con 4 sillas",
            "Table avec 4 chaises",
            "Tisch mit 4 Stühlen",
        ],
        "Mesa com 6 cadeiras" => [
            "Table with 6 chairs",
            "Mesa con 6 sillas",
            "Table avec 6 chaises",
            "Tisch mit 6 Stühlen",
        ],
        "Mesa redonda" => ["Round table", "Mesa redonda", "Table ronde", "Runder Tisch"],
        "Cadeira" => ["Chair", "Silla", "Chaise", "Stuhl"],
        "Aparador" => ["Sideboard", "Aparador", "Buffet", "Sideboard"],
        "Geladeira" => ["Fridge", "Frigorífico", "Réfrigérateur", "Kühlschrank"],
        "Fogão 4 bocas" => ["4-burner range", "Cocina de 4 fuegos", "Cuisinière 4 feux", "Herd mit 4 Flammen"],
        "Bancada com pia" => [
            "Countertop with sink",
            "Encimera con fregadero",
            "Plan de travail avec évier",
            "Arbeitsplatte mit Spüle",
        ],
        "Armário de cozinha (baixo)" => [
            "Base cabinet",
            "Mueble bajo de cocina",
            "Meuble bas de cuisine",
            "Unterschrank",
        ],
        "Armário aéreo" => ["Wall cabinet", "Mueble alto", "Meuble haut", "Oberschrank"],
        "Coifa" => ["Range hood", "Campana extractora", "Hotte", "Dunstabzug"],
        "Lava-louças" => ["Dishwasher", "Lavavajillas", "Lave-vaisselle", "Geschirrspüler"],
        "Micro-ondas" => ["Microwave", "Microondas", "Micro-ondes", "Mikrowelle"],
        "Cooktop 4 bocas (embutir)" => [
            "4-burner cooktop (built-in)",
            "Placa de 4 fuegos (encastrar)",
            "Table de cuisson 4 feux (encastrable)",
            "4-Flammen-Kochfeld (eingebaut)",
        ],
        "Cuba de inox (embutir)" => [
            "Stainless steel sink (built-in)",
            "Fregadero de acero inoxidable (encastrar)",
            "Évier inox (encastrable)",
            "Edelstahlspüle (eingebaut)",
        ],
        "Forno de embutir" => ["Built-in oven", "Horno empotrable", "Four encastrable", "Einbaubackofen"],
        "Ilha de cozinha" => ["Kitchen island", "Isla de cocina", "Îlot de cuisine", "Kücheninsel"],
        "Banqueta alta" => ["Bar stool", "Taburete alto", "Tabouret de bar", "Barhocker"],
        "Cama de casal" => ["Double bed", "Cama de matrimonio", "Lit double", "Doppelbett"],
        "Cama queen" => ["Queen bed", "Cama queen", "Lit queen", "Queen-Bett"],
        "Cama king" => ["King bed", "Cama king", "Lit king", "King-Bett"],
        "Cama de solteiro" => ["Single bed", "Cama individual", "Lit simple", "Einzelbett"],
        "Criado-mudo" => ["Bedside table", "Mesita de noche", "Table de chevet", "Nachttisch"],
        "Guarda-roupa" => ["Wardrobe", "Armario ropero", "Armoire", "Kleiderschrank"],
        "Cômoda" => ["Chest of drawers", "Cómoda", "Commode", "Kommode"],
        "Berço" => ["Cot", "Cuna", "Lit bébé", "Babybett"],
        "Vaso sanitário" => ["Toilet", "Inodoro", "WC", "WC"],
        "Gabinete com lavatório" => [
            "Vanity unit with basin",
            "Mueble con lavabo",
            "Meuble avec vasque",
            "Waschtisch mit Becken",
        ],
        "Box de chuveiro" => ["Shower tray", "Plato de ducha", "Receveur de douche", "Duschwanne"],
        "Box de vidro" => [
            "Glass shower screen",
            "Mampara de vidrio",
            "Paroi de douche en verre",
            "Glas-Duschabtrennung",
        ],
        "Banheira" => ["Bathtub", "Bañera", "Baignoire", "Badewanne"],
        "Máquina de lavar" => ["Washing machine", "Lavadora", "Lave-linge", "Waschmaschine"],
        "Secadora" => ["Tumble dryer", "Secadora", "Sèche-linge", "Wäschetrockner"],
        "Tanque" => ["Laundry sink", "Lavadero", "Bac à laver", "Ausgussbecken"],
        "Escrivaninha" => ["Desk", "Escritorio", "Bureau", "Schreibtisch"],
        "Cadeira de escritório" => ["Office chair", "Silla de oficina", "Chaise de bureau", "Bürostuhl"],
        "Porta" => ["Door", "Puerta", "Porte", "Tür"],
        "Porta dupla" => ["Double door", "Puerta doble", "Porte double", "Doppeltür"],
        "Porta de correr" => ["Sliding door", "Puerta corredera", "Porte coulissante", "Schiebetür"],
        "Portão de garagem" => ["Garage door", "Puerta de garaje", "Porte de garage", "Garagentor"],
        "Vão livre" => ["Opening", "Vano libre", "Ouverture libre", "Durchgang"],
        "Janela" => ["Window", "Ventana", "Fenêtre", "Fenster"],
        "Janela basculante" => ["Awning window", "Ventana abatible", "Fenêtre à soufflet", "Klappfenster"],
        "Porta-janela" => ["French door", "Puerta ventana", "Porte-fenêtre", "Fenstertür"],
        "Escada reta" => ["Straight stair", "Escalera recta", "Escalier droit", "Gerade Treppe"],
        "Pilar" => ["Column", "Pilar", "Poteau", "Stütze"],
        "Coluna redonda" => ["Round column", "Columna redonda", "Colonne ronde", "Runde Stütze"],
        "Sapata" => ["Footing", "Zapata", "Semelle", "Fundament"],
        "Viga / caibro" => ["Beam / rafter", "Viga / cabio", "Poutre / chevron", "Balken / Sparren"],
        "Painel" => ["Panel", "Panel", "Panneau", "Platte"],
        "Telha ondulada" => ["Corrugated sheet", "Chapa ondulada", "Tôle ondulée", "Wellblech"],
        "Gradil de ferro (guarda-corpo de barras verticais)" => [
            "Iron railing (vertical bar guard)",
            "Barandilla de hierro (barrotes verticales)",
            "Garde-corps en fer (barreaux verticaux)",
            "Eisengeländer (vertikale Stäbe)",
        ],
        "Guarda-corpo de vidro laminado com corrimão metálico" => [
            "Laminated glass guard with a metal handrail",
            "Barandilla de vidrio laminado con pasamanos metálico",
            "Garde-corps en verre feuilleté avec main courante métallique",
            "Verbundglasgeländer mit Metallhandlauf",
        ],
        "Fechamento de vidro da sacada (envidraçamento retrátil)" => [
            "Balcony glazing (retractable)",
            "Cerramiento de vidrio del balcón (acristalamiento retráctil)",
            "Fermeture vitrée du balcon (vitrage rétractable)",
            "Balkonverglasung (verschiebbar)",
        ],
        "Caixa" => ["Box", "Caja", "Caisse", "Kiste"],
        "Planta em vaso" => ["Potted plant", "Planta en maceta", "Plante en pot", "Topfpflanze"],
        "Floreira" => ["Planter", "Jardinera", "Jardinière", "Pflanzkübel"],
        "Muro / cerca" => ["Wall / fence", "Muro / valla", "Mur / clôture", "Mauer / Zaun"],
        "Piscina" => ["Swimming pool", "Piscina", "Piscine", "Schwimmbecken"],
        "Piscina oval" => ["Oval pool", "Piscina ovalada", "Piscine ovale", "Ovales Becken"],
        "Espreguiçadeira" => ["Sun lounger", "Tumbona", "Transat", "Liege"],
        "Churrasqueira" => ["Barbecue", "Barbacoa", "Barbecue", "Grill"],
        "Banco" => ["Bench", "Banco", "Banc", "Bank"],
        "Árvore" => ["Tree", "Árbol", "Arbre", "Baum"],
        "Carro" => ["Car", "Coche", "Voiture", "Auto"],
        "Tomada baixa (30 cm)" => [
            "Low socket (30 cm)",
            "Enchufe bajo (30 cm)",
            "Prise basse (30 cm)",
            "Steckdose unten (30 cm)",
        ],
        "Tomada média (1,10 m)" => [
            "Mid socket (1.10 m)",
            "Enchufe medio (1,10 m)",
            "Prise à mi-hauteur (1,10 m)",
            "Steckdose mitte (1,10 m)",
        ],
        "Tomada alta (2,20 m)" => [
            "High socket (2.20 m)",
            "Enchufe alto (2,20 m)",
            "Prise haute (2,20 m)",
            "Steckdose oben (2,20 m)",
        ],
        "Interruptor simples" => ["Single switch", "Interruptor simple", "Interrupteur simple", "Einfacher Schalter"],
        "Interruptor duplo" => ["Double switch", "Interruptor doble", "Interrupteur double", "Doppelschalter"],
        "Interruptor paralelo (three-way)" => [
            "Two-way switch (three-way)",
            "Conmutador (three-way)",
            "Va-et-vient (three-way)",
            "Wechselschalter",
        ],
        "Ponto de luz no teto" => [
            "Ceiling light point",
            "Punto de luz en el techo",
            "Point lumineux au plafond",
            "Deckenlichtpunkt",
        ],
        "Arandela (ponto de luz na parede)" => [
            "Wall light (light point on the wall)",
            "Aplique (punto de luz en la pared)",
            "Applique (point lumineux mural)",
            "Wandleuchte (Lichtpunkt an der Wand)",
        ],
        "Quadro de distribuição" => ["Consumer unit", "Cuadro eléctrico", "Tableau électrique", "Unterverteilung"],
        "Ponto de ar-condicionado" => [
            "Air conditioning point",
            "Punto de aire acondicionado",
            "Point de climatisation",
            "Klimapunkt",
        ],
        "Ponto para chuveiro elétrico" => [
            "Electric shower point",
            "Punto para calentador de ducha",
            "Point pour chauffe-eau de douche",
            "Durchlauferhitzer-Anschluss",
        ],
        "Tomada de dados / TV" => [
            "Data / TV socket",
            "Toma de datos / TV",
            "Prise données / TV",
            "Daten- / TV-Dose",
        ],
        "Ponto de rede RJ45 (Cat 6)" => [
            "RJ45 network point (Cat 6)",
            "Punto de red RJ45 (Cat 6)",
            "Prise réseau RJ45 (Cat 6)",
            "RJ45-Netzwerkpunkt (Cat 6)",
        ],
        "Ponto de TV (coaxial)" => [
            "TV point (coaxial)",
            "Punto de TV (coaxial)",
            "Prise TV (coaxiale)",
            "TV-Punkt (Koax)",
        ],
        "Ponto de Wi-Fi no teto (access point)" => [
            "Ceiling Wi-Fi point (access point)",
            "Punto de Wi-Fi en el techo (punto de acceso)",
            "Point Wi-Fi au plafond (point d'accès)",
            "WLAN-Punkt an der Decke (Access Point)",
        ],
        "Quadro de telecomunicações (rack / DG)" => [
            "Telecom panel (rack / MDF)",
            "Cuadro de telecomunicaciones (rack / RITI)",
            "Coffret de communication (baie / DTI)",
            "Telekom-Feld (Rack / MDF)",
        ],
        "Campainha" => ["Doorbell", "Timbre", "Sonnette", "Türklingel"],
        "Torre de tomadas retrátil de embutir (furo 60 mm, 3 tomadas, plugue)" => [
            "Retractable socket tower, built in (60 mm hole, 3 sockets, plug)",
            "Torre de enchufes retráctil empotrable (agujero 60 mm, 3 enchufes, clavija)",
            "Bloc de prises escamotable encastré (perçage 60 mm, 3 prises, fiche)",
            "Versenkbare Steckdosensäule, eingebaut (60-mm-Loch, 3 Steckdosen, Stecker)",
        ],
        "Torre de tomadas automática com indução (furo 85 mm, ligada à instalação)" => [
            "Automatic socket tower with induction (85 mm hole, wired in)",
            "Torre de enchufes automática con inducción (agujero 85 mm, conectada a la instalación)",
            "Bloc de prises automatique avec induction (perçage 85 mm, raccordé)",
            "Automatische Steckdosensäule mit Induktion (85-mm-Loch, fest verdrahtet)",
        ],
        "Torre de 4 tomadas + USB e indução (furo 100 mm)" => [
            "4-socket tower + USB and induction (100 mm hole)",
            "Torre de 4 enchufes + USB e inducción (agujero 100 mm)",
            "Bloc 4 prises + USB et induction (perçage 100 mm)",
            "4-fach-Säule + USB und Induktion (100-mm-Loch)",
        ],
        "Caixa de tomadas de mesa (4 tomadas, RJ45, USB, HDMI; recorte 120×335 mm)" => [
            "Desk socket box (4 sockets, RJ45, USB, HDMI; 120×335 mm cutout)",
            "Caja de enchufes de mesa (4 enchufes, RJ45, USB, HDMI; recorte 120×335 mm)",
            "Boîtier de prises de table (4 prises, RJ45, USB, HDMI ; découpe 120×335 mm)",
            "Tisch-Steckdosenbox (4 Steckdosen, RJ45, USB, HDMI; Ausschnitt 120×335 mm)",
        ],
        "Tomada de embutir em móvel (furo 35 mm)" => [
            "Socket built into furniture (35 mm hole)",
            "Enchufe empotrado en el mueble (agujero 35 mm)",
            "Prise encastrée dans le meuble (perçage 35 mm)",
            "Steckdose im Möbel (35-mm-Loch)",
        ],
        "Relé de automação (atrás do interruptor ou da luminária)" => [
            "Automation relay (behind the switch or the fixture)",
            "Relé de automatización (detrás del interruptor o de la luminaria)",
            "Relais domotique (derrière l'interrupteur ou le luminaire)",
            "Automationsrelais (hinter Schalter oder Leuchte)",
        ],
        "Interruptor inteligente" => [
            "Smart switch",
            "Interruptor inteligente",
            "Interrupteur connecté",
            "Smarter Schalter",
        ],
        "Dimmer" => ["Dimmer", "Regulador", "Variateur", "Dimmer"],
        "Sensor de presença de teto" => [
            "Ceiling presence sensor",
            "Sensor de presencia de techo",
            "Détecteur de présence au plafond",
            "Decken-Präsenzmelder",
        ],
        "Fechadura eletrônica" => [
            "Electronic lock",
            "Cerradura electrónica",
            "Serrure électronique",
            "Elektronisches Schloss",
        ],
        "Ponto de água fria" => [
            "Cold water point",
            "Punto de agua fría",
            "Point d'eau froide",
            "Kaltwasserpunkt",
        ],
        "Ponto de água quente" => [
            "Hot water point",
            "Punto de agua caliente",
            "Point d'eau chaude",
            "Warmwasserpunkt",
        ],
        "Ponto de esgoto" => ["Drain point", "Punto de desagüe", "Point d'évacuation", "Ablaufpunkt"],
        "Caixa sifonada 150×150×50 (ralo)" => [
            "Trapped gully 150×150×50 (drain)",
            "Sumidero sifónico 150×150×50",
            "Siphon de sol 150×150×50",
            "Geruchsverschlossener Bodenablauf 150×150×50",
        ],
        "Caixa sifonada 100×150×50 (ralo)" => [
            "Trapped gully 100×150×50 (drain)",
            "Sumidero sifónico 100×150×50",
            "Siphon de sol 100×150×50",
            "Geruchsverschlossener Bodenablauf 100×150×50",
        ],
        "Caixa sifonada 150×185×75 (ralo)" => [
            "Trapped gully 150×185×75 (drain)",
            "Sumidero sifónico 150×185×75",
            "Siphon de sol 150×185×75",
            "Geruchsverschlossener Bodenablauf 150×185×75",
        ],
        "Ralo sifonado 100 mm (fecho hídrico curto)" => [
            "Trapped floor drain 100 mm (shallow seal)",
            "Sumidero sifónico 100 mm (cierre hídrico corto)",
            "Bonde siphoïde 100 mm (garde d'eau courte)",
            "Geruchsverschlossener Bodenablauf 100 mm (flacher Geruchverschluss)",
        ],
        "Ralo seco 100 mm" => [
            "Dry floor drain 100 mm",
            "Sumidero seco 100 mm",
            "Bonde sèche 100 mm",
            "Trocken-Bodenablauf 100 mm",
        ],
        "Ralo linear 70 cm (sem sifão)" => [
            "Linear drain 70 cm (untrapped)",
            "Canaleta lineal 70 cm (sin sifón)",
            "Caniveau 70 cm (sans siphon)",
            "Rinnenablauf 70 cm (ohne Geruchverschluss)",
        ],
        "Ralo linear sifonado 70 cm" => [
            "Trapped linear drain 70 cm",
            "Canaleta lineal sifónica 70 cm",
            "Caniveau siphoïde 70 cm",
            "Rinnenablauf 70 cm mit Geruchverschluss",
        ],
        "Ralo de águas pluviais (varanda descoberta, terraço)" => [
            "Rainwater drain (open balcony, terrace)",
            "Sumidero de pluviales (balcón descubierto, terraza)",
            "Bonde d'eaux pluviales (balcon découvert, terrasse)",
            "Regenwasserablauf (offener Balkon, Terrasse)",
        ],
        "Registro" => ["Stopcock", "Llave de paso", "Robinet d'arrêt", "Absperrhahn"],
        "Caixa de gordura" => ["Grease trap", "Trampa de grasas", "Bac à graisse", "Fettabscheider"],
        "Grelha de ventilação permanente (gás)" => [
            "Permanent ventilation grille (gas)",
            "Rejilla de ventilación permanente (gas)",
            "Grille de ventilation permanente (gaz)",
            "Dauerlüftungsgitter (Gas)",
        ],
        "Tubo de ventilação" => ["Vent pipe", "Tubo de ventilación", "Colonne de ventilation", "Lüftungsrohr"],
        "Caixa de inspeção" => [
            "Inspection chamber",
            "Arqueta de inspección",
            "Regard de visite",
            "Revisionsschacht",
        ],
        "Hidrômetro" => ["Water meter", "Contador de agua", "Compteur d'eau", "Wasserzähler"],
        "Ponto de gás" => ["Gas point", "Punto de gas", "Point de gaz", "Gaspunkt"],
        // What walls are built of and what surfaces are finished with, as
        // the pickers list them.
        "Drywall 73 mm" => [
            "Drywall 73 mm",
            "Yeso laminado 73 mm",
            "Plaque de plâtre 73 mm",
            "Trockenbau 73 mm",
        ],
        "Drywall 95 mm" => [
            "Drywall 95 mm",
            "Yeso laminado 95 mm",
            "Plaque de plâtre 95 mm",
            "Trockenbau 95 mm",
        ],
        "Drywall 115 mm" => [
            "Drywall 115 mm",
            "Yeso laminado 115 mm",
            "Plaque de plâtre 115 mm",
            "Trockenbau 115 mm",
        ],
        "Drywall 120 mm chapa dupla" => [
            "Drywall 120 mm, double board",
            "Yeso laminado 120 mm, placa doble",
            "Plaque de plâtre 120 mm, double parement",
            "Trockenbau 120 mm, doppelte Beplankung",
        ],
        "Tijolo cerâmico 9 cm" => [
            "Clay brick 9 cm",
            "Ladrillo cerámico 9 cm",
            "Brique en terre cuite 9 cm",
            "Ziegel 9 cm",
        ],
        "Tijolo cerâmico 14 cm" => [
            "Clay brick 14 cm",
            "Ladrillo cerámico 14 cm",
            "Brique en terre cuite 14 cm",
            "Ziegel 14 cm",
        ],
        "Tijolo cerâmico 19 cm" => [
            "Clay brick 19 cm",
            "Ladrillo cerámico 19 cm",
            "Brique en terre cuite 19 cm",
            "Ziegel 19 cm",
        ],
        "Bloco de concreto 14 cm" => [
            "Concrete block 14 cm",
            "Bloque de hormigón 14 cm",
            "Bloc de béton 14 cm",
            "Betonstein 14 cm",
        ],
        "Bloco de concreto 19 cm" => [
            "Concrete block 19 cm",
            "Bloque de hormigón 19 cm",
            "Bloc de béton 19 cm",
            "Betonstein 19 cm",
        ],
        "Parede de concreto 10 cm" => [
            "Concrete wall 10 cm",
            "Muro de hormigón 10 cm",
            "Mur en béton 10 cm",
            "Betonwand 10 cm",
        ],
        "Concreto armado 15 cm" => [
            "Reinforced concrete 15 cm",
            "Hormigón armado 15 cm",
            "Béton armé 15 cm",
            "Stahlbeton 15 cm",
        ],
        "Steel frame 14 cm" => [
            "Steel frame 14 cm",
            "Steel frame 14 cm",
            "Ossature métallique 14 cm",
            "Stahlständer 14 cm",
        ],
        "Wood frame 10 cm" => [
            "Wood frame 10 cm",
            "Entramado de madera 10 cm",
            "Ossature bois 10 cm",
            "Holzständer 10 cm",
        ],
        "Divisória de vidro" => ["Glass partition", "Mampara de vidrio", "Cloison vitrée", "Glastrennwand"],
        "Montante 48 mm + 1 chapa de 12,5 mm em cada face" => [
            "48 mm stud + one 12.5 mm board each side",
            "Montante de 48 mm + 1 placa de 12,5 mm por cara",
            "Montant 48 mm + 1 plaque de 12,5 mm par face",
            "48-mm-Ständer + je 12,5 mm Platte je Seite",
        ],
        "Montante 70 mm + 1 chapa de 12,5 mm em cada face" => [
            "70 mm stud + one 12.5 mm board each side",
            "Montante de 70 mm + 1 placa de 12,5 mm por cara",
            "Montant 70 mm + 1 plaque de 12,5 mm par face",
            "70-mm-Ständer + je 12,5 mm Platte je Seite",
        ],
        "Montante 90 mm + 1 chapa de 12,5 mm em cada face" => [
            "90 mm stud + one 12.5 mm board each side",
            "Montante de 90 mm + 1 placa de 12,5 mm por cara",
            "Montant 90 mm + 1 plaque de 12,5 mm par face",
            "90-mm-Ständer + je 12,5 mm Platte je Seite",
        ],
        "Montante 70 mm + 2 chapas de 12,5 mm em cada face (acústica)" => [
            "70 mm stud + two 12.5 mm boards each side (acoustic)",
            "Montante de 70 mm + 2 placas de 12,5 mm por cara (acústica)",
            "Montant 70 mm + 2 plaques de 12,5 mm par face (acoustique)",
            "70-mm-Ständer + zwei 12,5-mm-Platten je Seite (Akustik)",
        ],
        "Bloco de 9 cm + reboco de 2,5 cm em cada face" => [
            "9 cm block + 2.5 cm render each side",
            "Bloque de 9 cm + enfoscado de 2,5 cm por cara",
            "Bloc de 9 cm + enduit de 2,5 cm par face",
            "9-cm-Stein + 2,5 cm Putz je Seite",
        ],
        "Bloco de 14 cm + reboco de 2,5 cm em cada face" => [
            "14 cm block + 2.5 cm render each side",
            "Bloque de 14 cm + enfoscado de 2,5 cm por cara",
            "Bloc de 14 cm + enduit de 2,5 cm par face",
            "14-cm-Stein + 2,5 cm Putz je Seite",
        ],
        "Bloco de 19 cm + reboco de 2,5 cm em cada face" => [
            "19 cm block + 2.5 cm render each side",
            "Bloque de 19 cm + enfoscado de 2,5 cm por cara",
            "Bloc de 19 cm + enduit de 2,5 cm par face",
            "19-cm-Stein + 2,5 cm Putz je Seite",
        ],
        "Bloco estrutural de 14 cm + revestimento de 1,5 cm em cada face" => [
            "14 cm structural block + 1.5 cm finish each side",
            "Bloque estructural de 14 cm + revestimiento de 1,5 cm por cara",
            "Bloc structurel de 14 cm + revêtement de 1,5 cm par face",
            "14-cm-Statikstein + 1,5 cm Belag je Seite",
        ],
        "Bloco estrutural de 19 cm + revestimento de 1,5 cm em cada face" => [
            "19 cm structural block + 1.5 cm finish each side",
            "Bloque estructural de 19 cm + revestimiento de 1,5 cm por cara",
            "Bloc structurel de 19 cm + revêtement de 1,5 cm par face",
            "19-cm-Statikstein + 1,5 cm Belag je Seite",
        ],
        "Concreto moldado in loco, sistema parede de concreto" => [
            "Cast-in-place concrete, concrete wall system",
            "Hormigón moldeado in situ, sistema de muro de hormigón",
            "Béton coulé en place, système de mur en béton",
            "Ortbeton, Betonwandsystem",
        ],
        "Parede estrutural de concreto armado" => [
            "Structural reinforced concrete wall",
            "Muro estructural de hormigón armado",
            "Mur structurel en béton armé",
            "Tragende Stahlbetonwand",
        ],
        "Perfil 90 mm + OSB e placa cimentícia/gesso" => [
            "90 mm profile + OSB and cement/plaster board",
            "Perfil de 90 mm + OSB y placa de cemento/yeso",
            "Profilé 90 mm + OSB et plaque ciment/plâtre",
            "90-mm-Profil + OSB und Zement-/Gipsplatte",
        ],
        "Estrutura de madeira com fechamento em chapas" => [
            "Timber frame closed with boards",
            "Estructura de madera cerrada con placas",
            "Ossature bois fermée par des panneaux",
            "Holzrahmen mit Platten geschlossen",
        ],
        "Vidro temperado de 10 mm" => [
            "10 mm toughened glass",
            "Vidrio templado de 10 mm",
            "Verre trempé de 10 mm",
            "10 mm Einscheibensicherheitsglas",
        ],
        "Madeira (réguas)" => ["Wood (planks)", "Madera (tablas)", "Bois (lames)", "Holz (Dielen)"],
        "Taco espinha de peixe" => [
            "Herringbone parquet",
            "Parqué en espiga",
            "Parquet à chevrons",
            "Fischgrätparkett",
        ],
        "Porcelanato / cerâmica" => [
            "Porcelain / ceramic tile",
            "Porcelánico / cerámica",
            "Grès cérame / carrelage",
            "Feinsteinzeug / Keramikfliese",
        ],
        "Azulejo metrô" => ["Subway tile", "Azulejo metro", "Carrelage métro", "Metrofliese"],
        "Tijolo aparente" => ["Exposed brick", "Ladrillo visto", "Brique apparente", "Sichtziegel"],
        "Pedra" => ["Stone", "Piedra", "Pierre", "Stein"],
        "Cimento queimado" => ["Polished cement", "Cemento pulido", "Béton ciré", "Geschliffener Zement"],
        "Mármore" => ["Marble", "Mármol", "Marbre", "Marmor"],
        "Carpete" => ["Carpet", "Moqueta", "Moquette", "Teppichboden"],
        "Grama" => ["Grass", "Césped", "Gazon", "Gras"],
        "Água (piscina)" => ["Water (pool)", "Agua (piscina)", "Eau (piscine)", "Wasser (Becken)"],
        "Deck de madeira" => ["Wood deck", "Tarima de madera", "Terrasse en bois", "Holzdeck"],
        "Projeto recuperado desta sessão." => [
            "Project recovered from this session.",
            "Proyecto recuperado de esta sesión.",
            "Projet récupéré de cette session.",
            "Projekt aus dieser Sitzung wiederhergestellt.",
        ],
        "No terminal:" => ["In a terminal:", "En el terminal:", "Dans un terminal :", "Im Terminal:"],
        "Os instaladores já registram sozinhos se o Claude Code estiver instalado." => [
            "The installers register it for you when Claude Code is installed.",
            "Los instaladores lo registran solos si Claude Code está instalado.",
            "Les installateurs l'enregistrent tout seuls si Claude Code est installé.",
            "Die Installer tragen es ein, wenn Claude Code installiert ist.",
        ],
        "Configurações → Desenvolvedor → Editar configuração" => [
            "Settings → Developer → Edit Config",
            "Configuración → Desarrollador → Editar configuración",
            "Réglages → Développeur → Modifier la configuration",
            "Settings → Developer → Edit Config",
        ],
        "Precisa do Node.js instalado para a ponte mcp-remote. Reinicie o Claude Desktop." => [
            "Needs Node.js for the mcp-remote bridge. Restart Claude Desktop.",
            "Necesita Node.js para el puente mcp-remote. Reinicie Claude Desktop.",
            "Nécessite Node.js pour le pont mcp-remote. Redémarrez Claude Desktop.",
            "Braucht Node.js für die mcp-remote-Brücke. Claude Desktop neu starten.",
        ],
        "Os instaladores já registram sozinhos se o Codex estiver instalado." => [
            "The installers register it for you when Codex is installed.",
            "Los instaladores lo registran solos si Codex está instalado.",
            "Les installateurs l'enregistrent tout seuls si Codex est installé.",
            "Die Installer tragen es ein, wenn Codex installiert ist.",
        ],
        "Depois abra o Gemini CLI e peça o projeto." => [
            "Then open Gemini CLI and ask for your project.",
            "Después abra Gemini CLI y pídale el proyecto.",
            "Ouvrez ensuite Gemini CLI et demandez-lui votre projet.",
            "Dann Gemini CLI öffnen und nach dem Projekt fragen.",
        ],
        "Use no modo agente do Copilot." => [
            "Use it in Copilot's agent mode.",
            "Úselo en el modo agente de Copilot.",
            "À utiliser dans le mode agent de Copilot.",
            "Im Agent-Modus von Copilot nutzen.",
        ],
        "Arquivo ~/.cursor/mcp.json" => [
            "File ~/.cursor/mcp.json",
            "Archivo ~/.cursor/mcp.json",
            "Fichier ~/.cursor/mcp.json",
            "Datei ~/.cursor/mcp.json",
        ],
        "Reinicie o Cursor depois de salvar." => [
            "Restart Cursor after saving.",
            "Reinicie Cursor después de guardar.",
            "Redémarrez Cursor après avoir enregistré.",
            "Cursor nach dem Speichern neu starten.",
        ],
        "Outro app" => ["Another app", "Otra app", "Une autre app", "Eine andere App"],
        "Nas configurações de MCP do app (Cline, Roo Code, Cherry Studio, LM Studio…)" => [
            "In the app's MCP settings (Cline, Roo Code, Cherry Studio, LM Studio…)",
            "En los ajustes de MCP de la app (Cline, Roo Code, Cherry Studio, LM Studio…)",
            "Dans les réglages MCP de l'app (Cline, Roo Code, Cherry Studio, LM Studio…)",
            "In den MCP-Einstellungen der App (Cline, Roo Code, Cherry Studio, LM Studio…)",
        ],
        "O que importa é o app falar MCP: o modelo pode ser qualquer um." => [
            "What matters is that the app speaks MCP: the model can be any of them.",
            "Lo que importa es que la app hable MCP: el modelo puede ser cualquiera.",
            "Ce qui compte, c'est que l'app parle MCP : le modèle peut être n'importe lequel.",
            "Wichtig ist, dass die App MCP spricht: das Modell darf eines beliebigen Anbieters sein.",
        ],
        "agora" => ["just now", "ahora", "à l'instant", "gerade eben"],
        "há {} s" => ["{} s ago", "hace {} s", "il y a {} s", "vor {} s"],
        "há {} min" => ["{} min ago", "hace {} min", "il y a {} min", "vor {} min"],
        "há {} h" => ["{} h ago", "hace {} h", "il y a {} h", "vor {} h"],
        "{} · {} chamadas" => ["{} · {} calls", "{} · {} llamadas", "{} · {} appels", "{} · {} Aufrufe"],
        "MCP · esperando sua IA" => [
            "MCP · waiting for your AI",
            "MCP · esperando su IA",
            "MCP · en attente de votre IA",
            "MCP · wartet auf Ihre KI",
        ],
        "IA: só no aplicativo" => [
            "AI: in the app only",
            "IA: solo en la aplicación",
            "IA : seulement dans l'application",
            "KI: nur in der App",
        ],
        "Conectar sua IA — clique para ver como" => [
            "Connect your AI — click to see how",
            "Conecte su IA — haga clic para ver cómo",
            "Connectez votre IA — cliquez pour voir comment",
            "KI verbinden — klicken, um zu sehen wie",
        ],
        "IA" => ["AI", "IA", "IA", "KI"],
        "{} conectado · {}" => ["{} connected · {}", "{} conectado · {}", "{} connecté · {}", "{} verbunden · {}"],
        "Nenhuma IA conectada ainda" => [
            "No AI connected yet",
            "Ninguna IA conectada todavía",
            "Aucune IA connectée pour l'instant",
            "Noch keine KI verbunden",
        ],
        "Conectar sua IA…" => ["Connect your AI…", "Conecte su IA…", "Connecter votre IA…", "KI verbinden…"],
        "Copiar o endereço do MCP" => [
            "Copy the MCP address",
            "Copiar la dirección del MCP",
            "Copier l'adresse MCP",
            "MCP-Adresse kopieren",
        ],
        "Endereço copiado" => ["Address copied", "Dirección copiada", "Adresse copiée", "Adresse kopiert"],
        "Conectar sua IA" => ["Connect your AI", "Conecte su IA", "Connecter votre IA", "KI verbinden"],
        "O editor abre uma porta MCP: a sua IA desenha aqui dentro, em centímetros, e você vê acontecer." => [
            "The editor opens an MCP port: your AI draws in here, in centimetres, and you watch it happen.",
            "El editor abre un puerto MCP: su IA dibuja aquí dentro, en centímetros, y usted lo ve ocurrir.",
            "L'éditeur ouvre un port MCP : votre IA dessine ici, en centimètres, et vous la regardez faire.",
            "Der Editor öffnet einen MCP-Port: Ihre KI zeichnet hier in Zentimetern, und Sie sehen es.",
        ],
        "Agora" => ["Right now", "Ahora mismo", "En ce moment", "Gerade"],
        "Servidor MCP ligado" => [
            "MCP server on",
            "Servidor MCP encendido",
            "Serveur MCP allumé",
            "MCP-Server an",
        ],
        "No navegador o editor roda sozinho: o MCP vive no aplicativo do computador." => [
            "In a browser the editor runs on its own: the MCP lives in the desktop app.",
            "En el navegador el editor funciona solo: el MCP vive en la aplicación de escritorio.",
            "Dans un navigateur, l'éditeur tourne seul : le MCP vit dans l'application de bureau.",
            "Im Browser läuft der Editor allein: MCP steckt in der Desktop-App.",
        ],
        "O servidor está desligado (--no-server): reabra o aplicativo sem essa opção." => [
            "The server is off (--no-server): reopen the app without that option.",
            "El servidor está apagado (--no-server): vuelva a abrir la aplicación sin esa opción.",
            "Le serveur est éteint (--no-server) : rouvrez l'application sans cette option.",
            "Der Server ist aus (--no-server): die App ohne diese Option neu öffnen.",
        ],
        "Nenhuma IA conectada ainda — siga os três passos abaixo." => [
            "No AI connected yet — the three steps below do it.",
            "Ninguna IA conectada todavía: los tres pasos de abajo lo resuelven.",
            "Aucune IA connectée — les trois étapes ci-dessous s'en chargent.",
            "Noch keine KI verbunden — die drei Schritte darunter erledigen es.",
        ],
        "1. Escolha o aplicativo de IA que você usa" => [
            "1. Pick the AI app you use",
            "1. Elija la app de IA que usa",
            "1. Choisissez l'app d'IA que vous utilisez",
            "1. Die KI-App wählen, die Sie nutzen",
        ],
        "2. Cole isto onde ele pede" => [
            "2. Paste this where it asks",
            "2. Pegue esto donde lo pide",
            "2. Collez ceci là où il le demande",
            "2. Das hier einfügen, wo sie danach fragt",
        ],
        "3. Peça alguma coisa" => [
            "3. Ask it for something",
            "3. Pídale algo",
            "3. Demandez-lui quelque chose",
            "3. Sie um etwas bitten",
        ],
        "Quantas paredes tem este projeto? Depois coloque uma janela de 120 cm na sala." => [
            "How many walls does this project have? Then put a 120 cm window in the living room.",
            "¿Cuántas paredes tiene este proyecto? Después ponga una ventana de 120 cm en el salón.",
            "Combien de murs a ce projet ? Ensuite, posez une fenêtre de 120 cm dans le salon.",
            "Wie viele Wände hat dieses Projekt? Dann ein 120-cm-Fenster ins Wohnzimmer setzen.",
        ],
        "Copiado" => ["Copied", "Copiado", "Copié", "Kopiert"],
        "Últimas chamadas" => ["Latest calls", "Últimas llamadas", "Derniers appels", "Letzte Aufrufe"],
        "Nada ainda. Assim que sua IA usar uma ferramenta, ela aparece aqui." => [
            "Nothing yet. The moment your AI uses a tool, it shows up here.",
            "Nada todavía. En cuanto su IA use una herramienta, aparece aquí.",
            "Rien encore. Dès que votre IA utilise un outil, cela apparaît ici.",
            "Noch nichts. Sobald Ihre KI ein Werkzeug nutzt, erscheint es hier.",
        ],
        "Baixar o aplicativo" => [
            "Download the app",
            "Descargar la aplicación",
            "Télécharger l'application",
            "App herunterladen",
        ],
        "{} conectado · {} chamadas · {}" => [
            "{} connected · {} calls · {}",
            "{} conectado · {} llamadas · {}",
            "{} connecté · {} appels · {}",
            "{} verbunden · {} Aufrufe · {}",
        ],
        "{} conectou — sua IA já pode desenhar aqui" => [
            "{} connected — your AI can draw in here now",
            "{} se conectó — su IA ya puede dibujar aquí",
            "{} s'est connecté — votre IA peut dessiner ici",
            "{} verbunden — Ihre KI kann hier jetzt zeichnen",
        ],
        "Registrar agora" => ["Register now", "Registrar ahora", "Enregistrer maintenant", "Jetzt eintragen"],
        "Registrando…" => ["Registering…", "Registrando…", "Enregistrement…", "Wird eingetragen…"],
        "Roda esse mesmo comando aqui, sem abrir o terminal." => [
            "Runs that same command from here, no terminal needed.",
            "Ejecuta ese mismo comando desde aquí, sin abrir el terminal.",
            "Exécute cette même commande d'ici, sans ouvrir de terminal.",
            "Führt denselben Befehl von hier aus, ohne Terminal.",
        ],
        "Registrado no {} — agora é só pedir" => [
            "Registered with {} — now just ask",
            "Registrado en {} — ahora solo pida",
            "Enregistré dans {} — il n'y a plus qu'à demander",
            "Bei {} eingetragen — jetzt einfach fragen",
        ],
        "Não deu para registrar: {}" => [
            "Could not register: {}",
            "No se pudo registrar: {}",
            "Impossible d'enregistrer : {}",
            "Eintragen fehlgeschlagen: {}",
        ],
        "Ligue e esta aba ganha um endereço que a sua IA alcança. O projeto não sai daqui: o servidor no meio só passa recados." => [
            "Switch it on and this tab gets an address your AI can reach. The project stays here: the server in between only passes messages.",
            "Actívelo y esta pestaña obtiene una dirección a la que su IA llega. El proyecto no sale de aquí: el servidor del medio solo pasa mensajes.",
            "Activez-le et cet onglet reçoit une adresse que votre IA peut joindre. Le projet reste ici : le serveur au milieu ne fait que passer les messages.",
            "Einschalten, und dieser Tab bekommt eine Adresse, die Ihre KI erreicht. Das Projekt bleibt hier: der Server dazwischen reicht nur Nachrichten weiter.",
        ],
        "Ligar o MCP nesta aba" => [
            "Switch the MCP on in this tab",
            "Activar el MCP en esta pestaña",
            "Activer le MCP dans cet onglet",
            "MCP in diesem Tab einschalten",
        ],
        "Ligando…" => ["Switching on…", "Activando…", "Activation…", "Wird eingeschaltet…"],
        "Esta aba está no ar para a sua IA" => [
            "This tab is reachable by your AI",
            "Esta pestaña está al alcance de su IA",
            "Cet onglet est joignable par votre IA",
            "Dieser Tab ist für Ihre KI erreichbar",
        ],
        "Quem tiver o endereço abaixo pode editar este projeto: trate como senha." => [
            "Anyone holding the address below can edit this project: treat it like a password.",
            "Quien tenga la dirección de abajo puede editar este proyecto: trátela como una contraseña.",
            "Quiconque détient l'adresse ci-dessous peut modifier ce projet : traitez-la comme un mot de passe.",
            "Wer die Adresse darunter hat, kann dieses Projekt bearbeiten: wie ein Passwort behandeln.",
        ],
        "Não deu para ligar: {}" => [
            "Could not switch it on: {}",
            "No se pudo activar: {}",
            "Impossible de l'activer : {}",
            "Einschalten fehlgeschlagen: {}",
        ],
        "Tentar de novo" => ["Try again", "Intentar de nuevo", "Réessayer", "Erneut versuchen"],
        "{} só no aplicativo do computador: aqui ele renderiza na CPU e travaria a aba" => [
            "{} only in the desktop app: here it renders on the CPU and would freeze the tab",
            "{} solo en la aplicación de escritorio: aquí renderiza en la CPU y congelaría la pestaña",
            "{} seulement dans l'application de bureau : ici il rend sur le CPU et figerait l'onglet",
            "{} nur in der Desktop-App: hier rendert es auf der CPU und würde den Tab einfrieren",
        ],
        "MCP desligado · ligar" => [
            "MCP off · switch on",
            "MCP apagado · activar",
            "MCP éteint · activer",
            "MCP aus · einschalten",
        ],
        "Desligar" => ["Switch off", "Apagar", "Éteindre", "Ausschalten"],
        "o relay recusou esta aba — ligue de novo" => [
            "the relay refused this tab — switch it on again",
            "el relay rechazó esta pestaña — actívelo de nuevo",
            "le relais a refusé cet onglet — rallumez-le",
            "das Relais hat diesen Tab abgelehnt — erneut einschalten",
        ],
        "Conectado como {}" => [
            "Signed in as {}",
            "Conectado como {}",
            "Connecté en tant que {}",
            "Angemeldet als {}",
        ],
        "Com este endereço, o Claude, o ChatGPT e outras IAs chegam nesta aba entrando com a sua conta." => [
            "With this address, Claude, ChatGPT and other AIs reach this tab by signing in with your account.",
            "Con esta dirección, Claude, ChatGPT y otras IA llegan a esta pestaña iniciando sesión con su cuenta.",
            "Avec cette adresse, Claude, ChatGPT et d'autres IA atteignent cet onglet en se connectant avec votre compte.",
            "Mit dieser Adresse erreichen Claude, ChatGPT und andere KIs diesen Tab, indem sie sich mit Ihrem Konto anmelden.",
        ],
        "Entrar para usar no Claude e no ChatGPT" => [
            "Sign in to use it in Claude and ChatGPT",
            "Iniciar sesión para usarlo en Claude y ChatGPT",
            "Se connecter pour l'utiliser dans Claude et ChatGPT",
            "Anmelden, um es in Claude und ChatGPT zu nutzen",
        ],
        "Com uma conta, a sua IA chega nesta aba por um endereço fixo, sem colar nada." => [
            "With an account, your AI reaches this tab at one fixed address, with nothing to paste.",
            "Con una cuenta, su IA llega a esta pestaña por una dirección fija, sin pegar nada.",
            "Avec un compte, votre IA atteint cet onglet à une adresse fixe, sans rien coller.",
            "Mit Konto erreicht Ihre KI diesen Tab unter einer festen Adresse, ohne etwas einzufügen.",
        ],
        "Fecha o endereço. A sua IA perde o acesso na hora; o projeto continua aqui." => [
            "Closes the address. Your AI loses access at once; the project stays here.",
            "Cierra la dirección. Su IA pierde el acceso al instante; el proyecto sigue aquí.",
            "Ferme l'adresse. Votre IA perd l'accès aussitôt ; le projet reste ici.",
            "Schließt die Adresse. Ihre KI verliert den Zugang sofort; das Projekt bleibt hier.",
        ],
        "Conectar sua IA (no navegador, liga e desliga)" => [
            "Connect your AI (in a browser, switches it on and off)",
            "Conecte su IA (en el navegador, activa y desactiva)",
            "Connecter votre IA (dans un navigateur, active et désactive)",
            "KI verbinden (im Browser schaltet das an und aus)",
        ],
        "3D" => ["3D", "3D", "3D", "3D"],
        "Catálogo · Planta · 3D (telas estreitas)" => [
            "Catalogue · Plan · 3D (narrow screens)",
            "Catálogo · Plano · 3D (pantallas estrechas)",
            "Catalogue · Plan · 3D (écrans étroits)",
            "Katalog · Grundriss · 3D (schmale Bildschirme)",
        ],
        "o servidor que leva a sua IA até esta aba não respondeu — no aplicativo do computador o MCP não precisa dele" => [
            "the server that carries your AI to this tab did not answer — in the desktop app the MCP needs no server",
            "el servidor que lleva su IA hasta esta pestaña no respondió — en la aplicación de escritorio el MCP no lo necesita",
            "le serveur qui relie votre IA à cet onglet n'a pas répondu — dans l'application de bureau, le MCP n'en a pas besoin",
            "der Server, der Ihre KI zu diesem Tab bringt, hat nicht geantwortet — in der Desktop-App braucht MCP keinen Server",
        ],
        "Disciplina, camadas e andares" => [
            "Discipline, layers and storeys",
            "Disciplina, capas y plantas",
            "Discipline, calques et étages",
            "Gewerk, Ebenen und Geschosse",
        ],
        "IA · esperando" => ["AI · waiting", "IA · esperando", "IA · en attente", "KI · wartet"],
        "Conectar IA" => ["Connect AI", "Conectar IA", "Connecter l'IA", "KI verbinden"],
        "{} conectado · {} chamadas" => [
            "{} connected · {} calls",
            "{} conectado · {} llamadas",
            "{} connecté · {} appels",
            "{} verbunden · {} Aufrufe",
        ],
        "Esperando a sua IA chegar" => [
            "Waiting for your AI to arrive",
            "Esperando a que llegue su IA",
            "En attente de votre IA",
            "Wartet, bis Ihre KI ankommt",
        ],
        "Ainda não dá para alcançar esta janela" => [
            "Nothing can reach this window yet",
            "Todavía nada puede alcanzar esta ventana",
            "Rien ne peut encore joindre cette fenêtre",
            "Dieses Fenster ist noch nicht erreichbar",
        ],
        "Escolha o aplicativo de IA que você usa" => [
            "Pick the AI app you use",
            "Elija la app de IA que usa",
            "Choisissez l'app d'IA que vous utilisez",
            "Die KI-App wählen, die Sie nutzen",
        ],
        "Cole isto onde ele pede" => [
            "Paste this where it asks",
            "Pegue esto donde lo pide",
            "Collez ceci là où il le demande",
            "Das hier einfügen, wo sie danach fragt",
        ],
        "Peça alguma coisa" => [
            "Ask it for something",
            "Pídale algo",
            "Demandez-lui quelque chose",
            "Sie um etwas bitten",
        ],
        "o endereço guardado não vale mais — ligue de novo" => [
            "the address kept here is no longer yours — switch it on again",
            "la dirección guardada ya no es suya — actívela de nuevo",
            "l'adresse gardée ici n'est plus la vôtre — activez-la à nouveau",
            "die hier gehaltene Adresse gehört Ihnen nicht mehr — erneut einschalten",
        ],
        "No ar, esperando a sua IA" => [
            "Up, waiting for your AI",
            "En el aire, esperando su IA",
            "En ligne, en attente de votre IA",
            "Bereit, wartet auf Ihre KI",
        ],
        "Quem tiver esse endereço edita este projeto: trate como senha." => [
            "Anyone with that address edits this project: treat it like a password.",
            "Quien tenga esa dirección edita este proyecto: trátela como una contraseña.",
            "Quiconque a cette adresse modifie ce projet : traitez-la comme un mot de passe.",
            "Wer diese Adresse hat, bearbeitet dieses Projekt: wie ein Passwort behandeln.",
        ],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names a new project starts with, and the catalog's, follow the
    /// window's language; one somebody typed does not. The English editor
    /// said "Versão 1", "Nova casa" and "Porta-janela".
    #[test]
    fn default_names_follow_the_language() {
        assert_eq!(shown_in("Versão 1", Lang::En), "Version 1");
        assert_eq!(shown_in("Versão 12", Lang::Es), "Versión 12");
        assert_eq!(shown_in("Versão 1 (cópia)", Lang::Fr), "Version 1 (copie)");
        assert_eq!(shown_in("Térreo", Lang::De), "Erdgeschoss");
        assert_eq!(shown_in("1º andar", Lang::De), "1. Obergeschoss");
        assert_eq!(shown_in("2º andar", Lang::En), "Floor 2");
        assert_eq!(shown_in("Nova casa", Lang::En), "New home");
        assert_eq!(shown_in("Porta-janela", Lang::En), "French door");
        assert_eq!(
            shown_in("Cadeira Ares (cópia)", Lang::Es),
            "Silla Ares (copia)"
        );
        // Portuguese is what the file holds, and a typed name is left alone.
        assert_eq!(shown_in("Versão 1", Lang::Pt), "Versão 1");
        assert_eq!(shown_in("Bloom Coffee", Lang::En), "Bloom Coffee");
        assert_eq!(shown_in("Versão final", Lang::En), "Versão final");
    }

    /// Every literal handed to `tr` has every other language, or the window
    /// silently falls back to Portuguese for whoever picked one of them —
    /// which is exactly how a new panel ships half translated.
    #[test]
    fn every_string_on_screen_is_translated() {
        fn sources(dir: &std::path::Path, into: &mut String) {
            for entry in std::fs::read_dir(dir).expect("src is readable") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    sources(&path, into);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    into.push_str(&std::fs::read_to_string(&path).expect("source"));
                }
            }
        }
        let mut code = String::new();
        sources(std::path::Path::new("src"), &mut code);
        // A literal handed to tr, whatever the wrapping and the trailing comma.
        let mut missing: Vec<String> = Vec::new();
        for needle in ["tr(", "fill("] {
            missing.extend(untranslated(&code, needle));
        }
        assert!(missing.is_empty(), "not translated: {missing:#?}");
    }

    /// Every literal handed to `call` — `tr(`, `fill(` — that no language
    /// speaks, or that some language leaves blank or without the `{}` the
    /// sentence needs.
    fn untranslated(code: &str, call: &str) -> Vec<String> {
        let mut missing: Vec<String> = Vec::new();
        let mut rest = code;
        while let Some(at) = rest.find(call) {
            // Match a function name, not the suffix of from_str or refill.
            let identifier_prefix = rest[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            rest = &rest[at + call.len()..];
            if identifier_prefix {
                continue;
            }
            let Some(open) = rest.find('"') else { break };
            if rest[..open].contains(|c: char| !c.is_whitespace()) {
                continue;
            }
            let body = &rest[open + 1..];
            let Some(close) = body.find('"') else { break };
            let (text, after) = (&body[..close], body[close + 1..].trim_start());
            if after.starts_with(',') || after.starts_with(')') {
                // The test's own probe, and words spelled the same everywhere.
                if ["Algo novo", "Plugins", "cooktop"].contains(&text) {
                    continue;
                }
                // A blank side is worse than no row: the label disappears.
                // A side without the `{}` of the key drops what goes in it.
                let places = text.matches("{}").count();
                match translations(text) {
                    Some(row)
                        if row
                            .iter()
                            .all(|t| !t.trim().is_empty() && t.matches("{}").count() == places) => {
                    }
                    _ => missing.push(text.to_owned()),
                }
            }
        }
        missing
    }

    #[test]
    fn translation_scan_matches_function_names_not_identifier_suffixes() {
        let code = "JsValue::from_str(\"neweraOperation\"); refill(\"buffer\"); i18n::tr(\"Missing UI text\"); tr(\"Salvar\"); fill(\"Missing format {}\", &[]);";
        assert_eq!(untranslated(code, "tr("), vec!["Missing UI text"]);
        assert_eq!(untranslated(code, "fill("), vec!["Missing format {}"]);
    }

    /// These run beside the tests that drive whole windows, which read the
    /// labels in Portuguese — so none of them touch the language the window
    /// is set to, they ask for a language by name.
    #[test]
    fn translates_to_the_chosen_language() {
        for (lang, saved) in [
            (Lang::Pt, "Salvar"),
            (Lang::En, "Save"),
            (Lang::Es, "Guardar"),
            (Lang::Fr, "Enregistrer"),
            (Lang::De, "Speichern"),
        ] {
            assert_eq!(say("Salvar", lang), saved);
            // A text nobody translated stays as written, in any language.
            assert_eq!(say("Algo novo", lang), "Algo novo");
        }
    }

    #[test]
    fn the_window_keeps_the_language_it_is_set_to() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), lang);
        }
    }

    #[test]
    fn fills_the_sentence_in_the_chosen_language() {
        assert_eq!(
            fill_in("Aberto: {}", Lang::Fr, &[&"casa.n3d"]),
            "Ouvert : casa.n3d"
        );
        assert_eq!(
            fill_in(
                "⚠ Não foi possível abrir {}: {}",
                Lang::Fr,
                &[&"casa.n3d", &"nada aqui"]
            ),
            "⚠ Impossible d'ouvrir casa.n3d : nada aqui"
        );
        assert_eq!(
            fill_in("Aberto: {}", Lang::Pt, &[&"casa.n3d"]),
            "Aberto: casa.n3d"
        );
        // Nothing to put in it, and nothing lost either.
        assert_eq!(fill_in("Salvo em {}", Lang::Pt, &[]), "Salvo em {}");
    }

    /// Every piece in the catalog is named in every language. Change the
    /// language and the sofa has to stop being a sofá — the catalog is the
    /// first thing anyone reads in this window.
    #[test]
    fn every_piece_of_the_catalog_is_named_in_every_language() {
        let mut missing: Vec<&str> = Vec::new();
        for item in newera_catalog::CATALOG {
            match translations(item.name) {
                Some(row) if row.iter().all(|t| !t.trim().is_empty()) => {}
                _ => missing.push(item.name),
            }
        }
        assert!(missing.is_empty(), "not translated: {missing:#?}");
    }

    #[test]
    fn reads_the_language_the_system_asks_for() {
        assert_eq!(Lang::from_locale("pt_BR.UTF-8"), Some(Lang::Pt));
        assert_eq!(Lang::from_locale("en_US.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::from_locale("es-419"), Some(Lang::Es));
        assert_eq!(Lang::from_locale("fr_CA"), Some(Lang::Fr));
        assert_eq!(Lang::from_locale("de_DE.UTF-8"), Some(Lang::De));
        assert_eq!(Lang::from_locale("C"), None);
        assert_eq!(Lang::from_locale(""), None);
    }

    #[test]
    fn only_english_writes_decimals_with_a_dot() {
        for lang in Lang::ALL {
            assert_eq!(lang.decimal_comma(), lang != Lang::En);
        }
    }
}
