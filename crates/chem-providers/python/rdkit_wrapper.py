from rdkit import Chem
from rdkit.Chem import Descriptors, inchi


def molecule_info(smiles: str) -> dict:
    mol = Chem.MolFromSmiles(smiles)
    if mol is None:
        raise ValueError("SMILES inválido")
    # Build atom list
    atoms = []
    for a in mol.GetAtoms():
        atoms.append({
            "index": a.GetIdx(),
            "atomic_number": a.GetAtomicNum(),
            "symbol": a.GetSymbol(),
            "implicit_h": a.GetNumImplicitHs(),
            "total_h": a.GetTotalNumHs()
        })
    bonds = []
    for b in mol.GetBonds():
        bt = b.GetBondType()
        bt_name = bt.name if hasattr(bt, 'name') else str(bt)
        order = 1
        if 'DOUBLE' in bt_name:
            order = 2
        elif 'TRIPLE' in bt_name:
            order = 3
        elif 'AROMATIC' in bt_name:
            order = 1
        bonds.append({
            "atom1": b.GetBeginAtomIdx(),
            "atom2": b.GetEndAtomIdx(),
            "order": order,
            "is_aromatic": b.GetIsAromatic()
        })

    # Identify substitution points as non-hydrogen atoms with at least
    # one hydrogen (implicit or explicit). This is a pragmatic heuristic for
    # where substitutions (e.g. R-group replacements) are possible.
    substitution_points = []
    for a in mol.GetAtoms():
        if a.GetAtomicNum() != 1 and a.GetTotalNumHs() > 0:
            substitution_points.append(a.GetIdx())

    info = {
        "smiles": Chem.MolToSmiles(mol),
        "inchi": inchi.MolToInchi(mol),
        "inchikey": inchi.MolToInchiKey(mol),
        "num_atoms": mol.GetNumAtoms(),
        "mol_weight": Descriptors.MolWt(mol),
        "mol_formula": Chem.rdMolDescriptors.CalcMolFormula(mol),
        "structure": {
            "atoms": atoms,
            "bonds": bonds,
            "substitution_points": substitution_points
        }
    }
    return info
