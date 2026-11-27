// Exporte en pseudo-C les fonctions dont le nom commence par un préfixe
// (« Mini_Bat »), puis celles qu'elles appellent (sur `depth` niveaux, sauf
// celles de la bibliothèque PS2 « sce* »), chacune avec la valeur des
// données qu'elle lit (entier et flottant) : les constantes du jeu y sont.
// Arguments : fichier de sortie ; préfixe ; profondeur des appels.
// Lecture seule : rien n'est exécuté.
// @category Dokuro
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.RefType;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.Symbol;

import java.io.File;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;

public class ExportMinigame extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args[0]);
        // Plusieurs préfixes possibles, séparés par « : » (cmd coupe les
        // arguments aux virgules).
        String prefix = args[1];
        String[] prefixes = prefix.split(":");
        int depth = Integer.parseInt(args[2]);

        // Les fonctions du préfixe, d'après la table de symboles (noms bruts
        // « Mini_BatProc__Fv » ou démanglés) ; une fonction est créée là où
        // l'analyse n'en a pas posé.
        Set<Function> chosen = new LinkedHashSet<>();
        List<Function> level = new ArrayList<>();
        for (Symbol s : currentProgram.getSymbolTable().getAllSymbols(true)) {
            boolean wanted = false;
            for (String p : prefixes) {
                wanted |= s.getName().startsWith(p);
            }
            if (!wanted || !s.getAddress().isMemoryAddress()) {
                continue;
            }
            Function f = getFunctionAt(s.getAddress());
            if (f == null && getInstructionAt(s.getAddress()) != null) {
                f = createFunction(s.getAddress(), s.getName());
            }
            if (f != null && !level.contains(f)) {
                level.add(f);
            }
        }
        println("symboles « " + prefix + " » : " + level.size() + " fonctions");
        chosen.addAll(level);
        for (int d = 0; d < depth; d++) {
            List<Function> next = new ArrayList<>();
            for (Function f : level) {
                for (Function callee : f.getCalledFunctions(monitor)) {
                    if (!callee.getName().startsWith("sce") && chosen.add(callee)) {
                        next.add(callee);
                    }
                }
            }
            level = next;
        }

        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        Memory memory = currentProgram.getMemory();
        out.getParentFile().mkdirs();
        try (PrintWriter w = new PrintWriter(out, "UTF-8")) {
            w.println("// Pseudo-C décompilé par Ghidra depuis SLPM_661.86 (Game ni Natta yo!");
            w.println("// Dokuro-chan, PS2), fonctions " + prefix + "* et leurs appels (" + depth + " niveaux).");
            for (Function f : chosen) {
                w.println();
                w.println("// " + f.getName() + " @ " + f.getEntryPoint());
                DecompileResults r = decompiler.decompileFunction(f, 120, monitor);
                w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// échec : " + r.getErrorMessage());
                // Données lues par la fonction : adresse, nom, valeur.
                Set<Address> data = new TreeSet<>();
                for (Instruction i : currentProgram.getListing().getInstructions(f.getBody(), true)) {
                    for (Reference ref : i.getReferencesFrom()) {
                        RefType t = ref.getReferenceType();
                        if (t.isData() && memory.contains(ref.getToAddress())) {
                            data.add(ref.getToAddress());
                        }
                    }
                }
                for (Address a : data) {
                    Symbol s = getSymbolAt(a);
                    String name = s == null ? "?" : s.getName();
                    try {
                        int bits = memory.getInt(a);
                        w.printf("//   donnée %s %s : %d (0x%08x), flottant %s%n", a, name, bits, bits, Float.intBitsToFloat(bits));
                    } catch (Exception e) {
                        w.printf("//   donnée %s %s : illisible%n", a, name);
                    }
                }
            }
        }
        println("exporté : " + chosen.size() + " fonctions dans " + out);
    }
}
