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


def _bond_type_from_order(order: int):
    if order == 1:
        return Chem.BondType.SINGLE
    if order == 2:
        return Chem.BondType.DOUBLE
    if order == 3:
        return Chem.BondType.TRIPLE
    raise ValueError(f"Unsupported bond order: {order}")


def fuse_molecules(smiles_a: str, smiles_b: str, atom_a: int, atom_b: int, bond_order: int = 1) -> dict:
    """
    Fuses two molecules by creating a bond between atom_a (in first) and atom_b (in second).

    Parameters
    ----------
    smiles_a : str
        SMILES of the first (principal) molecule.
    smiles_b : str
        SMILES of the second (substituent) molecule.
    atom_a : int
        Atom index in the first molecule to connect.
    atom_b : int
        Atom index in the second molecule to connect.
    bond_order : int
        Bond order (1,2,3).

    Returns
    -------
    dict
        The same structure returned by molecule_info for the fused molecule.
    """
    mol_a = Chem.MolFromSmiles(smiles_a)
    mol_b = Chem.MolFromSmiles(smiles_b)
    if mol_a is None:
        raise ValueError(f"SMILES inválido (mol_a): {smiles_a}")
    if mol_b is None:
        raise ValueError(f"SMILES inválido (mol_b): {smiles_b}")
    if atom_a < 0 or atom_a >= mol_a.GetNumAtoms():
        raise ValueError(f"atom_a fuera de rango: {atom_a}")
    if atom_b < 0 or atom_b >= mol_b.GetNumAtoms():
        raise ValueError(f"atom_b fuera de rango: {atom_b}")
    combo = Chem.CombineMols(mol_a, mol_b)
    offset = mol_a.GetNumAtoms()
    em = Chem.EditableMol(combo)
    em.AddBond(int(atom_a), int(atom_b) + offset, _bond_type_from_order(int(bond_order)))
    fused = em.GetMol()
    Chem.SanitizeMol(fused)
    smiles_fused = Chem.MolToSmiles(fused)
    return molecule_info(smiles_fused)

