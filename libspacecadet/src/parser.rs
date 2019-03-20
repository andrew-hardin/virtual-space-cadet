/// Having never written a compiler or taken a course on compilers,
/// this is a rough first pass at compiling key-syntax into something
/// that's even marginally friendly to work with.
///
/// Key's are specified via the following syntax:
///
///   identifier()
///
/// The parenthesis are used to pass arguments:
///
///   identifier(arg1, arg2, ...)
///
/// Keys without arguments can omit the parenthesis:
///
///   identifier   ==   identifier()
///
/// Some keys expect other keys as arguments. As expected, these
/// arguments are nested:
///
///   identifier(arg1, identifier2(arg2))
///
/// All the "expected" syntax rules apply, for example:
///
/// identifier(,arg1) // illegal due to misplaced comma

use crate::keys::*;


/// Raw token represent the basic building blocks of key syntax.
#[derive(Debug)]
enum RawToken<'a> {
    Identifier(&'a str),
    StartArgumentCollection,
    EndArgumentCollection,
    Comma,
}

impl<'a> RawToken<'a> {
    /// Create raw tokens given a string constant. Parsing errors are
    /// returned via the Result.
    pub fn create(v: &str) -> Result<Vec<RawToken>, String> {
        let ans = RawToken::parse(v);
        RawToken::verify_first_is_identifier(&ans)?;
        RawToken::verify_ordering(&ans)?;
        RawToken::verify_collection_starts_and_ends(&ans)?;
        Ok(ans)
    }

    fn parse(v: &str) -> Vec<RawToken> {
        assert!(!v.is_empty());

        let mut ans = Vec::new();
        let delimited : Vec<(usize, &str)> = v.match_indices(|c| "(),".contains(c)).collect();
        match delimited.len() {
            0 => {
                // Just a token - no commas or parens.
                // Add a pair of synthetic () tokens.
                // Later code expects tokens to start like "identifier(optional_args)".
                ans.push(RawToken::Identifier(v));
                ans.push(RawToken::StartArgumentCollection);
                ans.push(RawToken::EndArgumentCollection);
            }
            _ => {
                // Check if the first delimiter is also the first character.
                // If not, then we have an identifier to add.
                let first_position = delimited[0].0;
                if first_position != 0 {
                    ans.push(RawToken::Identifier(&v[..first_position]));
                }

                // Loop though each token and the string on its left.
                for i in 0..delimited.len() {
                    ans.push(match delimited[i].1 {
                        "(" => RawToken::StartArgumentCollection,
                        ")" => RawToken::EndArgumentCollection,
                        "," => RawToken::Comma,
                        _ => panic!("Logic error")
                    });
                    let start_idx = delimited[i].0 + 1;
                    let stop_idx = if i < delimited.len() - 1 { delimited[i + 1].0 } else { delimited.len() };
                    if start_idx < stop_idx {
                        ans.push(RawToken::Identifier(&v[start_idx..stop_idx]));
                    }
                }
            }
        }
        return ans;
    }

    fn verify_first_is_identifier(tokens: &Vec<RawToken>) -> Result<(), String> {

        // First element must be an identifier.
        match tokens.get(0) {
            Some(t) => match t {
                RawToken::Identifier(_) => Ok(()),
                _ => Err("Expected an identifier at the beginning.".to_string())
            }
            None => Err("Collection of tokens is empty.".to_string())
        }?;

        // The second element must be a "(".
        match tokens.get(1) {
            Some(t) => match t {
                RawToken::StartArgumentCollection => Ok(()),
                _ => Err("The second token can only be \"(\" or \"\".".to_string())
            }
            None => Err("Missing a token after the first identifier.".to_string())
        }
    }

    fn verify_ordering(tokens: &Vec<RawToken>) -> Result<(), String> {
        // This is the complete list of allowed syntax:
        //   identifier
        //   identifier + (
        //   identifier + ,
        //   identifier + )identifier
        //   )
        //   ) + ,
        //   ) + )
        //   ( + )
        //   ( + identifier
        //   , + identifier
        for i in 0..tokens.len() {
            let last = i == tokens.len() - 1;
            match tokens[i] {
                RawToken::Identifier(_) => {
                    // There's nothing really to verify here.
                    // An identifier can be before any of the other tokens.
                },
                RawToken::StartArgumentCollection => {
                    // Argument start can be followed by any other token except
                    // nothing or another argument start.
                    if last {
                        return Err("The \"(\" character must be followed by another token.".to_string());
                    }
                    match tokens[i + 1] {
                        RawToken::StartArgumentCollection => {
                            return Err("The \"(\" token can't be followed by another \"(\".".to_string());
                        },
                        RawToken::Comma => {
                            return Err("The \"(\" token can't be followed by a \",\".".to_string());
                        },
                        _ => ()
                    }
                },
                RawToken::EndArgumentCollection => {
                    if !last {
                        match tokens[i + 1] {
                            RawToken::Identifier(_) => {
                                return Err("The \")\" token can't be followed immediately by an identifier.".to_string());
                            },
                            _ => ()
                        }
                    }
                }
                RawToken::Comma => {
                    if !last {
                        match tokens[i + 1] {
                            RawToken::Identifier(_) => (),
                            _ => return Err("The \",\" token can only be followed by an identifier.".to_string())
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn verify_collection_starts_and_ends(tokens: &Vec<RawToken>) -> Result<(), String> {
        // Scan from left to right counting the number of start and stop tokens.
        // Start and stop need to cancel each other out.
        let mut start_count = 0;
        for i in tokens {
            match i {
                RawToken::StartArgumentCollection => start_count += 1,
                RawToken::EndArgumentCollection => {
                    start_count -= 1;
                    if start_count < 0  {
                        return Err("Missing a \"(\" token.".to_string())
                    }
                }
                _ => ()
            }
        }

        if start_count > 0 {
            Err("Missing a \")\" token.".to_string())
        } else {
            Ok(())
        }
    }
}

/// Simplified tokens are one step up from raw tokens - they form
/// higher level concepts like functions.
#[derive(Debug)]
enum SimplifiedToken<'a> {
    Function(&'a str),
    Argument(&'a str),
    EndFunction,
}

impl<'a> SimplifiedToken<'a> {
    pub fn simplify(items: Vec<RawToken>) -> Vec<SimplifiedToken> {
        let mut ans = vec![];
        for i in 0..items.len() {
            let v = &items[i];
            let next = items.get(i + 1);

            match v {
                RawToken::Comma => (),
                RawToken::StartArgumentCollection => (),
                RawToken::EndArgumentCollection => ans.push(SimplifiedToken::EndFunction),
                RawToken::Identifier(v) => {
                    ans.push(match next.unwrap_or(&RawToken::Comma) {
                        RawToken::StartArgumentCollection => SimplifiedToken::Function(v),
                        _ => SimplifiedToken::Argument(v)
                    });
                }
            }
        }
        ans
    }
}


/// Precursor to keys - hierarchical collection of identifiers and optional arguments (all strings).
#[derive(Debug)]
pub struct ParsedKeyTree<'a> {
    pub identifier: &'a str,
    pub args: Vec<ParsedKeyTree<'a>>
}

impl<'a> ParsedKeyTree<'a> {
    pub fn create(v: &str) -> Result<ParsedKeyTree, String> {

        // Convert the string into a set of tokens, then simplify.
        let raw = RawToken::create(v)?;
        let tokens = SimplifiedToken::simplify(raw);

        // Take those tokens and assemble them into a key tree.
        let mut idx = 0;
        let args = ParsedKeyTree::convert_to_tree_recursive(&tokens, &mut idx);
        Ok(args)
    }

    fn convert_to_tree_recursive(vals: &Vec<SimplifiedToken<'a>>, idx: &mut usize) -> ParsedKeyTree<'a> {

        // Setup a default answer.
        let mut ans = ParsedKeyTree {
            identifier: "",
            args: vec![]
        };

        // The first item will always be a function.
        match vals[*idx] {
            SimplifiedToken::Function(a) => ans.identifier = a,
            _ => unreachable!()
        }
        *idx += 1;

        // Loop until we find an EndFunction token.
        while *idx < vals.len() {
            match vals[*idx] {
                SimplifiedToken::EndFunction => break,
                SimplifiedToken::Function(_) => ans.args.push(ParsedKeyTree::convert_to_tree_recursive(vals, idx)),
                SimplifiedToken::Argument(v) => ans.args.push(ParsedKeyTree { identifier: v, args: vec![] })
            }
            *idx += 1;
        }
        ans
    }
}


pub fn convert_tokens_to_key(v: &ParsedKeyTree) -> Result<Box<KeyCode>, String> {

    // Try to parse the key tree using every known key.
    // Wish there was a reflection based alternative for listing
    // every possible key.
    type Converter = fn(&ParsedKeyTree) -> Result<Box<KeyCode>, String>;
    let converters: [Converter; 11] = [
        |x| { Ok(Box::new(NormalKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(TransparentKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(OpaqueKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(MacroKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(ToggleLayerKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(MomentarilyEnableLayerKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(ActivateLayerKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(HoldEnableLayerPressKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(OneShotLayer::from_tokens(x)?)) },
        |x| { Ok(Box::new(WrappedKey::from_tokens(x)?)) },
        |x| { Ok(Box::new(SpaceCadet::from_tokens(x)?)) },
    ];

    for i in converters.into_iter() {
        let v = i(&v);
        match v {
            Err(_) => (),
            Ok(v) => {
                return Ok(v)
            }
        }
    }

    Err("Failed to convert key tree into a key.".to_string())
}