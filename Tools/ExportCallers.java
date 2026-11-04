// Exporte en pseudo-C les fonctions qui appellent les fonctions nommées
// (« Mini_BatProc »…), chacune une fois. Arguments : fichier de sortie, puis
// les noms. Lecture seule : rien n'est exécuté.
// @category Dokuro
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Symbol;

import java.io.File;
import java.io.PrintWriter;
import java.util.LinkedHashSet;
import java.util.Set;

public class ExportCallers extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        Set<Function> callers = new LinkedHashSet<>();
        for (int i = 1; i < args.length; i++) {
            for (Symbol s : currentProgram.getSymbolTable().getSymbols(args[i])) {
                Function f = getFunctionAt(s.getAddress());
                if (f == null) {
                    continue;
                }
                for (Function c : f.getCallingFunctions(monitor)) {
                    println(args[i] + " <- " + c.getName() + " @ " + c.getEntryPoint());
                    callers.add(c);
                }
            }
        }
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try (PrintWriter w = new PrintWriter(new File(args[0]), "UTF-8")) {
            for (Function f : callers) {
                w.println();
                w.println("// " + f.getName() + " @ " + f.getEntryPoint());
                DecompileResults r = decompiler.decompileFunction(f, 120, monitor);
                w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// échec : " + r.getErrorMessage());
            }
        }
    }
}
