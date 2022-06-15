#![allow(dead_code)]
mod policy;
mod table;


fn main()
{
    let table = match table::Table::from_file()
    {
        Ok(table) => table,
        Err(_) =>
        {
            println!("No datasets are being tracked");
            return;
        },
    };

    println!("{}", table);
}
