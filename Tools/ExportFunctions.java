// Exporte la liste de toutes les fonctions de l'exécutable, une par ligne
// (séparées par des tabulations) : adresse, nom, taille en octets, adresses
// des fonctions qu'elle appelle, et 1 si une donnée pointe dessus (une table
// de fonctions, un gestionnaire). Sert au bilan de couverture
// (coverage.py). Argument : fichier de sortie. Lecture seule.
// @category Dokuro
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.PrintWriter;
import java.util.StringJoiner;

public class ExportFunctions extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String out = getScriptArgs()[0];
        try (PrintWriter w = new PrintWriter(new File(out), "UTF-8")) {
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                StringJoiner callees = new StringJoiner(",");
                for (Function c : f.getCalledFunctions(monitor)) {
                    callees.add(c.getEntryPoint().toString());
                }
                boolean data = false;
                for (Reference r : getReferencesTo(f.getEntryPoint())) {
                    if (!r.getReferenceType().isCall() && !r.getReferenceType().isJump()) {
                        data = true;
                        break;
                    }
                }
                w.println(f.getEntryPoint() + "\t" + f.getName() + "\t" + f.getBody().getNumAddresses() + "\t" + callees + "\t" + (data ? 1 : 0));
            }
        }
    }
}
