// About Octet Draw: the app's mark, its version, who made it, and the credits the data and libraries' licences ask for.
import { invoke } from "@tauri-apps/api/core";
import { Heart } from "@phosphor-icons/react";
import { Dialog } from "./Dialog";
import icon from "../src-tauri/icons/app-icon.svg";
import { version } from "../package.json";

export function About({ onClose }: { onClose: () => void }) {
  return (
    <Dialog title="About Octet Draw" onClose={onClose} plain>
      <div className="about">
        <img src={icon} alt="" width={96} height={96} draggable={false} />
        <p className="about-name">Octet Draw</p>
        <p className="about-version">Version {version}</p>
        <p className="about-made">
          Made with <Heart size={12} className="about-heart" aria-label="love" /> in Manchester and Cambridge by{" "}
          <button className="about-link" onClick={() => void invoke("open_link", { name: "author" }).catch(() => undefined)}>Sam Ellis</button>.
        </p>
        <p className="about-credits">
          Thanks to my wonderful pals for beta testing: Elliot Thaw, Dylan Davies‑Jones, James Baron, Elena Lahr, Seb
          Kinzie and Ying Hou.
        </p>
        <p className="about-credits">
          Chemistry by Indigo (Apache-2.0). Names read by OPSIN (MIT). NMR shifts contain information from nmrshiftdb2
          (www.nmrshiftdb.org), which is made available here under the nmrshiftdb2 Database License.
        </p>
        {/* Every licence notice Octet ships (scripts/licences.mjs), in the system's text app. */}
        <p className="about-credits">
          <button className="about-link" onClick={() => void invoke("open_licences").catch(() => undefined)}>Licences</button>
        </p>
      </div>
    </Dialog>
  );
}
