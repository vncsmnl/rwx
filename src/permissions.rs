#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionBits {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl PermissionBits {
    pub fn from_bits(bits: u32) -> Self {
        Self {
            read: (bits & 4) != 0,
            write: (bits & 2) != 0,
            execute: (bits & 1) != 0,
        }
    }

    pub fn to_bits(&self) -> u32 {
        let mut bits = 0;
        if self.read {
            bits |= 4;
        }
        if self.write {
            bits |= 2;
        }
        if self.execute {
            bits |= 1;
        }
        bits
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    pub owner: PermissionBits,
    pub group: PermissionBits,
    pub others: PermissionBits,
    pub setuid: bool,
    pub setgid: bool,
    pub sticky: bool,
}

impl FilePermissions {
    pub fn from_mode(mode: u32) -> Self {
        Self {
            owner: PermissionBits::from_bits((mode >> 6) & 7),
            group: PermissionBits::from_bits((mode >> 3) & 7),
            others: PermissionBits::from_bits(mode & 7),
            setuid: (mode & 0o4000) != 0,
            setgid: (mode & 0o2000) != 0,
            sticky: (mode & 0o1000) != 0,
        }
    }

    pub fn to_mode(&self) -> u32 {
        let mut mode = 0;
        mode |= self.owner.to_bits() << 6;
        mode |= self.group.to_bits() << 3;
        mode |= self.others.to_bits();

        if self.setuid {
            mode |= 0o4000;
        }
        if self.setgid {
            mode |= 0o2000;
        }
        if self.sticky {
            mode |= 0o1000;
        }
        mode
    }

    pub fn to_octal(&self) -> String {
        let mode = self.to_mode();
        format!("{:04o}", mode)
    }

    pub fn to_symbolic(&self, is_dir: bool) -> String {
        let mut s = String::with_capacity(10);
        s.push(if is_dir { 'd' } else { '-' });

        // Owner
        s.push(if self.owner.read { 'r' } else { '-' });
        s.push(if self.owner.write { 'w' } else { '-' });
        s.push(match (self.owner.execute, self.setuid) {
            (true, true) => 's',
            (false, true) => 'S',
            (true, false) => 'x',
            (false, false) => '-',
        });

        // Group
        s.push(if self.group.read { 'r' } else { '-' });
        s.push(if self.group.write { 'w' } else { '-' });
        s.push(match (self.group.execute, self.setgid) {
            (true, true) => 's',
            (false, true) => 'S',
            (true, false) => 'x',
            (false, false) => '-',
        });

        // Others
        s.push(if self.others.read { 'r' } else { '-' });
        s.push(if self.others.write { 'w' } else { '-' });
        s.push(match (self.others.execute, self.sticky) {
            (true, true) => 't',
            (false, true) => 'T',
            (true, false) => 'x',
            (false, false) => '-',
        });

        s
    }

    pub fn from_octal_str(octal: &str) -> Option<Self> {
        let val = u32::from_str_radix(octal, 8).ok()?;
        if val > 0o7777 {
            return None;
        }
        Some(Self::from_mode(val))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_conversions() {
        let perm = FilePermissions::from_mode(0o755);
        assert_eq!(perm.to_octal(), "0755");
        assert_eq!(perm.to_symbolic(false), "-rwxr-xr-x");
        assert_eq!(perm.to_symbolic(true), "drwxr-xr-x");

        let special = FilePermissions::from_mode(0o4755);
        assert_eq!(special.to_octal(), "4755");
        assert_eq!(special.to_symbolic(false), "-rwsr-xr-x");

        let sticky = FilePermissions::from_mode(0o1777);
        assert_eq!(sticky.to_octal(), "1777");
        assert_eq!(sticky.to_symbolic(true), "drwxrwxrwt");

        let none = FilePermissions::from_mode(0o000);
        assert_eq!(none.to_octal(), "0000");
        assert_eq!(none.to_symbolic(false), "----------");
    }

    #[test]
    fn test_from_octal_str() {
        let perm = FilePermissions::from_octal_str("755").unwrap();
        assert_eq!(perm.to_mode(), 0o755);

        let perm_four = FilePermissions::from_octal_str("4755").unwrap();
        assert_eq!(perm_four.to_mode(), 0o4755);

        assert!(FilePermissions::from_octal_str("888").is_none());
        assert!(FilePermissions::from_octal_str("10000").is_none());
    }
}
