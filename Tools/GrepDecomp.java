// Décompile toutes les fonctions et exporte celles dont le pseudo-C contient
// l'un des motifs (expressions régulières), pour retrouver les adresses
// calculées que Ghidra ne relie pas. Arguments : fichier de sortie, puis
// les motifs. Lecture seule.
// @category Dokuro
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

import java.io.File;
import java.io.PrintWriter;
import java.util.regex.Pattern;

public class GrepDecomp extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        Pattern[] patterns = new Pattern[args.length - 1];
        for (int i = 1; i < args.length; i++) {
            patterns[i - 1] = Pattern.compile(args[i]);
        }
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try (PrintWriter w = new PrintWriter(new File(args[0]), "UTF-8")) {
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                DecompileResults r = decompiler.decompileFunction(f, 60, monitor);
                if (!r.decompileCompleted()) {
                    continue;
                }
                String c = r.getDecompiledFunction().getC();
                for (Pattern p : patterns) {
                    if (p.matcher(c).find()) {
                        println(p.pattern() + " : " + f.getName() + " @ " + f.getEntryPoint());
                        w.println();
                        w.println("// " + f.getName() + " @ " + f.getEntryPoint());
                        w.println(c);
                        break;
                    }
                }
            }
        }
    }
}
