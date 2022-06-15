use std::cmp::Ordering;

impl<C> Lattice for Box<C>
where
    C: Fn(&LatticeValue, &LatticeValue) -> Ordering,
{
    fn compare(&self, rhs: &LatticeValue, lhs: &LatticeValue) -> std::cmp::Ordering
    {
        self(rhs, lhs)
    }
}

pub fn create_lattice(ltype: &LatticeType) -> impl Lattice
{
    match ltype
    {
        &LatticeType::LinearNumber =>
        {
            Box::new(Box::new(|rhs: &LatticeValue, lhs: &LatticeValue| {
                // In the future, this _won't_ be irrefutable
                #[allow(irrefutable_let_patterns)]
                if let (LatticeValue::Number(v1), LatticeValue::Number(v2)) = (rhs, lhs)
                {
                    i64::cmp(v1, v2)
                }
                else
                {
                    panic!("Lattice values are not of the same type");
                }
            }))
        },
    }
}

pub trait Lattice
{
    fn compare(&self, rhs: &LatticeValue, lhs: &LatticeValue) -> std::cmp::Ordering;
}

#[derive(Debug, Clone)]
pub enum LatticeType
{
    LinearNumber,
}


impl std::fmt::Display for LatticeType
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            &LatticeType::LinearNumber => write!(f, "linear"),
        }
    }
}

pub type LatticePair = (LatticeType, LatticeValue);
pub fn lattice_pair_default() -> LatticePair
{
    let lattice_type = LatticeType::LinearNumber;
    let val = lattice_type.default();
    (lattice_type, val)
}

pub fn lattice_pair_strictest() -> LatticePair
{
    (LatticeType::LinearNumber, LatticeValue::Number(1))
}

impl LatticeType
{
    pub fn default(&self) -> LatticeValue
    {
        match self
        {
            LatticeType::LinearNumber => LatticeValue::Number(3),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LatticeValue
{
    Number(i64),
}

impl LatticeValue
{
    pub fn from_string(ltype: &LatticeType, s: &str) -> LatticeValue
    {
        match ltype
        {
            &LatticeType::LinearNumber => LatticeValue::Number(s.parse::<i64>().unwrap()),
        }
    }
}

impl std::fmt::Display for LatticeValue
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        match self
        {
            &LatticeValue::Number(n) => write!(f, "{}", n),
        }
    }
}
