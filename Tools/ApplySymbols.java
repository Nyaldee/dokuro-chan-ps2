// Nomme les fonctions du programme d'après un fichier de symboles (sortie de
// tools/dwarf1_symbols.py : « func <nom> <début hex> <fin hex> ») : la
// fonction est créée à son début si l'analyse n'en a pas posé.
// Argument : fichier de symboles.
// @category Dokuro
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

import java.nio.file.Files;
import java.nio.file.Paths;

public class ApplySymbols extends GhidraScript {
    @Override
    protected void run() throws Exception {
        int named = 0, created = 0;
        for (String line : Files.readAllLines(Paths.get(getScriptArgs()[0]))) {
            String[] p = line.trim().split(" ");
            if (p.length < 3 || !p[0].equals("func")) {
                continue;
            }
            Address at = toAddr(Long.parseLong(p[2], 16));
            Function f = getFunctionAt(at);
            if (f == null) {
                disassemble(at);
                f = createFunction(at, p[1]);
                if (f == null) {
                    continue;
                }
                created++;
            }
            f.setName(p[1], SourceType.IMPORTED);
            named++;
        }
        println("fonctions nommées : " + named + " (dont créées : " + created + ")");
    }
}
