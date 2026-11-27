// Exporte en pseudo-C les fonctions qui lisent ou écrivent les adresses
// données (hexadécimal, sur une plage de 8 octets chacune). Arguments :
// fichier de sortie, puis les adresses. Lecture seule.
// @category Dokuro
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.PrintWriter;
import java.util.LinkedHashSet;
import java.util.Set;

public class ExportRefs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        Set<Function> users = new LinkedHashSet<>();
        for (int i = 1; i < args.length; i++) {
            Address base = toAddr(Long.parseLong(args[i], 16));
            for (int k = 0; k < 8; k++) {
                for (Reference r : getReferencesTo(base.add(k))) {
                    Function f = getFunctionContaining(r.getFromAddress());
                    if (f != null && users.add(f)) {
                        println(args[i] + " <- " + f.getName() + " @ " + f.getEntryPoint() + " (" + r.getReferenceType() + ")");
                    }
                }
            }
        }
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try (PrintWriter w = new PrintWriter(new File(args[0]), "UTF-8")) {
            for (Function f : users) {
                w.println();
                w.println("// " + f.getName() + " @ " + f.getEntryPoint());
                DecompileResults r = decompiler.decompileFunction(f, 120, monitor);
                w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// échec : " + r.getErrorMessage());
            }
        }
    }
}
