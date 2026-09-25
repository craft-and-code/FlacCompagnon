// Plain-language introductions complement the detailed method pages.
// Each pair is [French, English]; the audio itself is never changed by a check.
export const explainers = {
  upscaling: [
    [
      "Un fichier 24 bits peut contenir un son qui n’utilise réellement que 16 bits. C’est comme agrandir une photo : un contenant plus grand ne recrée pas les détails absents.",
      "A 24-bit file can contain sound that really uses only 16 bits. Like enlarging a photo, a bigger container cannot recreate missing detail.",
    ],
    [
      "16-bit indique une précision mesurée exactement ; ≈16-bit signale une estimation. Des traitements peuvent masquer ces indices. Le résultat ne permet pas de dater ou de retracer l’enregistrement.",
      "16-bit is an exact measured depth; ≈16-bit is an estimate. Processing can hide these clues. The result cannot date or reconstruct the recording history.",
    ],
  ],
  upsampling: [
    [
      "Un fichier annoncé à 96 kHz dispose de place pour des fréquences bien plus hautes qu’un CD. Si cette zone reste vide, le son peut provenir d’une source à fréquence d’échantillonnage plus basse.",
      "A file labelled 96 kHz has room for frequencies far above those on a CD. If that region stays empty, the sound may come from a lower-rate source.",
    ],
    [
      "Upsampled est un indice à examiner avec le spectrogramme. Un enregistrement naturellement pauvre en aigus ou volontairement filtré peut donner le même résultat.",
      "Upsampled is a cue to inspect alongside the spectrogram. A naturally dark or deliberately filtered recording can produce the same result.",
    ],
  ],
  transcoding: [
    [
      "Convertir un MP3 ou un AAC en FLAC conserve ce qu’il reste du son, sans restaurer les détails perdus. Cette analyse recherche des traces du calcul effectué par ces codecs.",
      "Converting an MP3 or AAC to FLAC preserves the remaining sound without restoring lost detail. This analysis searches for traces of those codecs’ calculations.",
    ],
    [
      "Transcoded signifie qu’une signature statistique a été détectée. Clean signifie qu’aucun détecteur n’a levé d’alerte, sans garantir l’origine du fichier. La calibration reste préliminaire.",
      "Transcoded means a statistical signature was detected. Clean means no detector raised a finding, without guaranteeing the file’s origin. Calibration remains preliminary.",
    ],
  ],
  "spectral-cutoff": [
    [
      "La coupure indique jusqu’où le fichier contient une énergie sonore mesurable dans son spectre moyen. Imaginez une carte allant des graves aux aigus : elle repère la fin de la zone occupée.",
      "The cutoff shows how far measurable sound energy extends in the average spectrum. Think of a map from bass to treble: it locates the edge of the occupied region.",
    ],
    [
      "Une coupure basse n’est pas une preuve de compression avec perte. Une coupure haute peut simplement venir du bruit. Ce nombre sert à orienter l’inspection du spectrogramme.",
      "A low cutoff does not prove lossy compression. A high cutoff may simply come from noise. This number helps guide spectrogram inspection.",
    ],
  ],
  "fake-stereo": [
    [
      "Un fichier peut avoir deux canaux et jouer pratiquement la même chose à gauche et à droite. On parle de dual mono : le contenant est stéréo, le signal est essentiellement mono.",
      "A file can have two channels playing almost the same thing on the left and right. This is dual mono: a stereo container with an essentially mono signal.",
    ],
    [
      "C’est normal pour de nombreux anciens disques ou enregistrements parlés. L’indication décrit les canaux ; elle n’accuse pas le fichier d’être défectueux.",
      "This is normal for many older records or spoken recordings. The indication describes the channels; it does not label the file defective.",
    ],
  ],
  "stereo-polarity": [
    [
      "Si un canal pousse pendant que l’autre tire, ils peuvent se compenser lorsqu’on les mélange en mono. Cette analyse observe leur relation sur toute la piste.",
      "If one channel pushes while the other pulls, they can cancel when mixed to mono. This analysis examines their relationship across the whole track.",
    ],
    [
      "Près de +1 : ils évoluent ensemble. Près de −1 : ils s’opposent. Avant de corriger quoi que ce soit, écoutez en mono : des effets stéréo intentionnels peuvent aussi créer une opposition.",
      "Near +1: the channels move together. Near −1: they oppose each other. Listen in mono before making a correction: intentional stereo effects can also create opposition.",
    ],
  ],
  "local-phase": [
    [
      "Une moyenne sur tout le morceau peut cacher un problème de quelques secondes. Ici, le son est observé par petites fenêtres, puis séparément dans les graves, médiums, hauts médiums et aigus.",
      "A whole-track average can hide a problem lasting a few seconds. Here, sound is examined in short windows, then separately in bass, midrange, upper midrange and treble.",
    ],
    [
      "Les colonnes montrent la corrélation la plus basse. Une valeur négative invite à écouter le passage indiqué en mono. Le pourcentage porte sur les fenêtres mesurables, pas sur la durée totale du morceau.",
      "The columns show the lowest correlation. A negative value invites a mono check of the indicated passage. The percentage describes eligible windows, not the track’s total duration.",
    ],
  ],
  "stereo-balance": [
    [
      "La balance compare le niveau moyen des deux canaux. Elle aide à repérer une piste qui penche à gauche ou à droite, sans confondre ce décalage avec la largeur stéréo.",
      "Balance compares the average levels of both channels. It helps spot a track leaning left or right, without confusing that difference with stereo width.",
    ],
    [
      "L +6.0 dB signifie que le canal gauche a un niveau RMS supérieur de 6 dB. Un mixage peut être volontairement asymétrique ; le nombre n’impose aucune correction.",
      "L +6.0 dB means the left channel’s RMS level is 6 dB higher. A mix can be deliberately asymmetric; the number does not prescribe a correction.",
    ],
  ],
  "high-frequency-stereo": [
    [
      "Les aigus semblent-ils beaucoup plus centrés que le reste du son ? HF Stereo compare la différence entre gauche et droite dans les aigus à celle d’une bande de référence.",
      "Does the treble seem much more centred than the rest of the sound? HF Stereo compares the left/right difference in treble with that in a reference band.",
    ],
    [
      "Un nombre plus négatif signifie moins de différence stéréo par rapport au signal central. L’indice reste expérimental : des choix de mixage peuvent imiter une réduction de largeur liée à un codec.",
      "A more negative number means less stereo difference relative to the centre signal. This cue remains experimental: mixing choices can resemble codec-related narrowing.",
    ],
  ],
  "dc-offset": [
    [
      "Une onde sonore oscille normalement autour de zéro. Un décalage continu la pousse en moyenne vers le haut ou vers le bas, ce qui peut réduire la place disponible avant saturation.",
      "A sound wave normally oscillates around zero. DC offset shifts its average up or down, which can reduce the space available before clipping.",
    ],
    [
      "La valeur est le plus grand décalage absolu parmi les canaux, en pourcentage. 2.000 signifie 2 % de la pleine échelle. Un très court extrait peut avoir une moyenne décalée sans panne matérielle.",
      "The value is the largest absolute channel offset, as a percentage. 2.000 means 2% of full scale. A very short excerpt can have a shifted mean without a hardware fault.",
    ],
  ],
  clipping: [
    [
      "Quand le signal atteint le plafond numérique, les sommets peuvent devenir plats. FlacCompagnon compte les suites d’échantillons à pleine échelle ou très proches.",
      "When a signal reaches the digital ceiling, its peaks can flatten. FlacCompagnon counts runs of samples at or very near full scale.",
    ],
    [
      "Le nombre indique des plateaux possibles, pas leur audibilité. Un fichier sans plateau peut encore dépasser le plafond entre ses échantillons : c’est le rôle du true peak.",
      "The count indicates possible plateaus, not their audibility. A file with no plateau can still exceed the ceiling between its samples: that is what true peak estimates.",
    ],
  ],
  "true-peak": [
    [
      "Le son lu par un appareil ne saute pas simplement d’un point enregistré à l’autre : une onde est reconstruite entre eux. Elle peut dépasser le plafond même si aucun point enregistré ne le fait.",
      "Playback does not simply jump between stored points: a waveform is reconstructed between them. It may exceed the ceiling even when no stored point does.",
    ],
    [
      "Au-dessus de 0 dBTP, la reconstruction estimée dépasse la pleine échelle. C’est une indication de marge disponible, pas une preuve de distorsion audible sur tous les appareils.",
      "Above 0 dBTP, the estimated reconstruction exceeds full scale. This describes headroom, not proof of audible distortion on every device.",
    ],
  ],
  "dynamic-range": [
    [
      "Le DR compare les crêtes aux niveaux moyens des passages forts. Il donne une idée de la place laissée aux attaques et aux contrastes à l’intérieur de ces passages.",
      "DR compares peaks with the average levels of loud passages. It gives a sense of the room left for attacks and contrasts within those passages.",
    ],
    [
      "Un DR élevé traduit un écart plus grand entre crête et moyenne. Il ne suffit pas à choisir le meilleur mastering. La variation entre passages calmes et forts est décrite autrement par la LRA.",
      "A high DR means a larger gap between peak and average. It is not enough to choose the best master. Variation between quiet and loud passages is described differently by LRA.",
    ],
  ],
  "integrated-loudness": [
    [
      "Deux morceaux peuvent avoir la même crête et sembler très différents en volume. Le LUFS intégré résume le loudness de tout le programme en tenant compte de la sensibilité aux fréquences et des silences.",
      "Two tracks can have the same peak yet sound very different in level. Integrated LUFS summarises whole-programme loudness while accounting for frequency sensitivity and silence.",
    ],
    [
      "−14 LUFS est plus fort que −23 LUFS. Comparez des programmes complets de même nature. Cette mesure n’est ni une note de qualité ni une consigne automatique de normalisation.",
      "−14 LUFS is louder than −23 LUFS. Compare complete programmes of similar scope. This is neither a quality grade nor an automatic normalization instruction.",
    ],
  ],
  "momentary-loudness": [
    [
      "Le LUFS-M max repère la fenêtre de 400 millisecondes la plus forte. Il répond à une question locale : quel court passage concentre le plus de loudness ?",
      "LUFS-M max locates the loudest 400-millisecond window. It answers a local question: which brief passage contains the most loudness?",
    ],
    [
      "La valeur est le maximum du fichier, pas le niveau au point de lecture. Survolez la cellule pour retrouver la fenêtre. Les fichiers de moins de 400 ms n’ont pas de résultat.",
      "The value is the file maximum, not the current playback level. Hover over the cell to find the window. Files shorter than 400 ms have no result.",
    ],
  ],
  "short-term-loudness": [
    [
      "Le LUFS-S max repère les trois secondes les plus fortes. Il lisse davantage les accents brefs que la mesure momentanée de 400 ms et décrit mieux un passage soutenu.",
      "LUFS-S max locates the loudest three seconds. It smooths brief accents more than the 400 ms momentary measurement and better describes a sustained passage.",
    ],
    [
      "Un accent très bref peut donner un LUFS-M élevé et un LUFS-S bien plus bas. Un fichier de moins de trois secondes n’a pas de valeur S. La LRA mesure une variation, pas ce maximum.",
      "A brief accent can produce high LUFS-M and much lower LUFS-S. A file shorter than three seconds has no S value. LRA measures variation, not this maximum.",
    ],
  ],
  "loudness-range": [
    [
      "La LRA décrit l’écart entre les passages plutôt calmes et plutôt forts, après exclusion des silences et des extrêmes. Elle aide à comparer les variations de volume dans le temps.",
      "LRA describes the gap between relatively quiet and relatively loud passages after excluding silence and extremes. It helps compare level variation over time.",
    ],
    [
      "Plus la valeur en LU est grande, plus le loudness varie. Deux pistes peuvent partager le même LUFS intégré et avoir des LRA différentes. Sous une minute, l’estimation est moins représentative.",
      "The larger the value in LU, the more loudness varies. Two tracks can share integrated LUFS yet have different LRA. Below one minute, the estimate is less representative.",
    ],
  ],
  impulses: [
    [
      "Une impulsion est une pointe très brève qui peut ressembler à un clic. L’analyse cherche des formes isolées et conserve leurs positions pour faciliter l’écoute et l’inspection.",
      "An impulse is a very brief spike that may resemble a click. The analysis searches for isolated shapes and retains locations for listening and inspection.",
    ],
    [
      "0 signifie qu’aucun candidat n’a été trouvé par cette règle. Une percussion peut être comptée ; un clic discret peut être manqué. Les impulsions sont différentes des plateaux de clipping.",
      "0 means this rule found no candidate. A percussion hit can be counted; a quiet click can be missed. Impulses are different from clipping plateaus.",
    ],
  ],
  dropouts: [
    [
      "Un dropout est ici un court trou de silence numérique exact au milieu d’un signal actif. Il peut signaler des données audio manquantes, mais aussi un montage ou une coupure volontaire.",
      "A dropout here is a short gap of exact digital silence within active audio. It can indicate missing audio, but also an edit or deliberate mute.",
    ],
    [
      "Utilisez les positions pour écouter chaque candidat. Un trou rempli de bruit, un fondu ou une coupure longue ne correspond pas à cette règle. Le comptage se fait séparément par canal.",
      "Use the locations to audition candidates. A noise-filled gap, fade or long interruption does not match this rule. Events are counted separately per channel.",
    ],
  ],
  "flac-md5": [
    [
      "Le FLAC peut contenir une signature de son audio d’origine. FlacCompagnon décode le son, recalcule sa signature et compare les deux pour repérer une éventuelle altération.",
      "FLAC can store a signature of its original audio. FlacCompagnon decodes the sound, recomputes the signature and compares both to detect possible alteration.",
    ],
    [
      "MD5 OK signifie que l’audio correspond à la signature. Cela ne garantit pas qu’il provient d’un enregistrement sans perte. Modifier les tags ou la pochette ne change pas cette signature audio.",
      "MD5 OK means the audio matches the signature. It does not guarantee a lossless recording source. Editing tags or cover art does not change this audio signature.",
    ],
  ],
  fingerprints: [
    [
      "Une empreinte est une étiquette calculée à partir de tous les octets du fichier. Elle permet de reconnaître une copie identique même si son nom ou son dossier change.",
      "A fingerprint is a label calculated from every byte in a file. It helps recognize an identical copy even after its name or folder changes.",
    ],
    [
      "Le MD5 du fichier inclut les tags et images : une retouche les modifie donc aussi. La signature audio FLAC est différente. Ces empreintes ne reconnaissent pas la même chanson après réencodage.",
      "File MD5 includes tags and images, so editing them changes it too. The FLAC audio signature is different. These fingerprints do not recognize the same song after re-encoding.",
    ],
  ],
  "dsd-pcm-source": [
    [
      "Un fichier DSD peut avoir été créé à partir de PCM. L’analyse cherche une coupure très nette à un endroit compatible avec une ancienne source CD ou 48 kHz.",
      "A DSD file may have been made from PCM. The analysis searches for a sharp boundary consistent with an earlier CD-rate or 48 kHz source.",
    ],
    [
      "C’est un indice spectral, pas un historique de fabrication. FFmpeg est nécessaire pour lire le contenu DSD ; sans lui, seules les informations du conteneur sont disponibles.",
      "This is a spectral clue, not a production history. FFmpeg is needed to read DSD content; without it, only container information is available.",
    ],
  ],
  "dsd-heritage": [
    [
      "Certains transferts DSD vers PCM conservent une remontée de bruit au-delà des fréquences audibles. Cette analyse cherche cette forme dans les fichiers PCM à haute fréquence d’échantillonnage.",
      "Some DSD-to-PCM transfers retain a noise rise beyond audible frequencies. This analysis searches for that shape in high-rate PCM files.",
    ],
    [
      "Le badge Hi-Res (DSD source) décrit une forme compatible avec ce transfert. Un autre procédé peut produire la même forme ; un filtrage peut au contraire la faire disparaître.",
      "The Hi-Res (DSD source) badge describes a shape consistent with that transfer. Another process can produce the same shape; filtering can remove it.",
    ],
  ],
};
