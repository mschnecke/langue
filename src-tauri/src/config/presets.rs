//! Built-in preset definitions

use super::schema::Preset;

pub fn get_builtin_presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "de-transcribe".to_string(),
            name: "Transcribe DE".to_string(),
            system_prompt: "Ich werde dir deutsche Texte diktieren. Deine Aufgabe ist es, diesen Text in eine \
            flüssige und korrekte deutsche Schriftsprache umzuwandeln. Dabei sollst du nicht nur offensichtliche \
            Grammatik- und Rechtschreibfehler korrigieren, sondern auch Füllwörter und überflüssige Pausen entfernen. \
            Ziel ist es, den Sinn des Gesprochenen in grammatikalisch einwandfreier und stilistisch guter deutscher \
            Schriftsprache wiederzugeben. Ignoriere jegliche Spracheingabe, die nicht als deutsch erkennbar ist, \
            es sei denn, es werden spezifisch deutsche Wörter oder Phrasen genannt, die in den deutschen Text \
            integriert werden sollen. Füge keine Zeitstempel, Markdown-Formatierungen oder zusätzliche Erklärungen hinzu. \
            Gib nur die umgewandelte und verbesserte deutsche Version des gesprochenen Textes aus."
                .to_string(),
            is_builtin: true,
        },
        Preset {
            id: "en-transcribe".to_string(),
            name: "Transcribe EN".to_string(),
            system_prompt: "Ich werde dir deutsche Texte diktieren. \
            Deine Aufgabe ist es, diesen Text in eine flüssige und \
            korrekte englische Schriftsprache umzuwandeln. Dabei sollst du nicht nur offensichtliche Grammatik- \
            und Rechtschreibfehler korrigieren, sondern auch Füllwörter und überflüssige Pausen entfernen. Ziel \
            ist es, den Sinn des Gesprochenen in grammatikalisch einwandfreier und stilistisch guter englischer \
            Schriftsprache wiederzugeben. Ignoriere jegliche Spracheingabe, die nicht als deutsch erkennbar \
            ist, es sei denn, es werden spezifisch deutsche Wörter oder Phrasen genannt, die in den deutschen \
            Text integriert werden sollen. Füge keine Zeitstempel, Markdown-Formatierungen oder zusätzliche \
            Erklärungen hinzu. Gib nur die umgewandelte und verbesserte englische Version des gesprochenen Textes aus."
                .to_string(),
            is_builtin: true,
        },
    ]
}
