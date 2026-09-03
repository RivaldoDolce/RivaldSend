import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  fr: {
    translation: {
      dropTitle: "Déposez vos fichiers ici",
      dropSubtitle: "ou cliquez pour parcourir",
      selectFiles: "Sélectionner des fichiers",
      peers: "Appareils à proximité",
      noPeers: "Aucun appareil trouvé",
      noPeersDesc: "Assurez-vous que les deux appareils sont sur le même réseau Wi-Fi",
      send: "Envoyer",
      history: "Historique",
      settings: "Paramètres",
      transfer: "Transfert",
      pairing: "Appairage",
      downloadDir: "Dossier de téléchargement",
      network: "Réseau",
      darkMode: "Mode sombre",
      language: "Langue",
      pairingCode: "Code d'appairage",
      scanQr: "Scanner le QR code",
      enterIp: "Ou saisir IP / port manuellement",
    },
  },
  en: {
    translation: {
      dropTitle: "Drop files here",
      dropSubtitle: "or click to browse",
      selectFiles: "Select files",
      peers: "Nearby devices",
      noPeers: "No devices found",
      noPeersDesc: "Make sure both devices are on the same Wi-Fi network",
      send: "Send",
      history: "History",
      settings: "Settings",
      transfer: "Transfer",
      pairing: "Pairing",
      downloadDir: "Download folder",
      network: "Network",
      darkMode: "Dark mode",
      language: "Language",
      pairingCode: "Pairing code",
      scanQr: "Scan QR code",
      enterIp: "Or enter IP / port manually",
    },
  },
};

void i18n.use(initReactI18next).init({
  resources,
  lng: "fr",
  fallbackLng: "fr",
  interpolation: { escapeValue: false },
});

export default i18n;
