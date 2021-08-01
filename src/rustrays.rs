#![feature(rustc_private)]
//#![feature(collections)]
//#![feature(rand)]
//#![feature(step_by)]
//#![feature(iterator_step_by)]
extern crate rustc_serialize;
extern crate getopts;
//extern crate collections;
extern crate rand;
use getopts::*;
use rustc_serialize::json;
use rustc_serialize::json::Json;
use rustc_serialize::json::ToJson;
use std::str::FromStr;
use std::time;
use std::path;
use std::io::Read;
use std::io::Write;
use std::fs::File;
use std::ops::*;
use rand::*;
use std::vec::Vec;
use std::str;
use std::collections::BTreeMap;
type Art = Vec<String>;
type Objects = Vec<Vector>;

#[derive(Clone)]
struct Racy<T>(T);

unsafe impl<T: 'static> Send for Racy<T> {}

fn main() {
    let opts = Options::new();
    let art_filename = if opts.art_filename == "ART" && !opts.home.is_empty() {
                        opts.home.clone() + "/ART"
                       }
                       else {
                        opts.art_filename.clone()
                       };
                       
    let art_path:&path::Path = path::Path::new(&art_filename);
    let art = read_art(&mut File::open(&art_path).unwrap() as &mut Read);
    let objects = make_objects(&art);
    let objects = Racy(&objects as *const Objects);
    let image_size = (opts.mega_pixels * 1000.0 * 1000.0).sqrt() as usize;
    let mut bytes = from_elem(image_size * image_size * 3,0u8);
    let bytes = Racy(&mut bytes as *mut Vec<u8>);
    let mut res = ResultJ::new(opts.times as usize);
    let mut rng = rand::isaac::IsaacRng::new_unseeded();
    
    for i_times in 0..opts.times {
        let t0 = time::Instant::now();
        let mut results = Vec::new();
        for i in 0..opts.procs {
            let objects = objects.clone();
            let bytes = bytes.clone();
            let opts = opts.clone();
            let seed = Rand::rand(&mut rng);
            results.push(std::thread::spawn(move || unsafe {
                let Racy(objects) = objects;
                let Racy(bytes) = bytes;
                let g = !Vector::new_args(-3.1,-16.,1.9);
                let a = !(Vector::new_args(0.,0.,1.)^g)*0.002;
                let b = !(g^a)*0.002;
                let c = (a+b)*-256.+g;
                let ar = 512.0 / image_size as f32;
                let orig0 = Vector::new_args(-5.0,16.0,8.0);
                let mut seed = seed;
                for y in (i as usize..image_size).step_by(opts.procs as usize) {
                    if (y as usize) >= image_size { break }
                    let mut k = (image_size - (y as usize)-1) * image_size * 3;
                    let mut x = image_size;
                    while x > 0 {
                        x -= 1;
                        let mut p = Vector::new_args(13.0,13.0,13.0);
                        for _ in 0..64 {
                            let t = a*((rnd(&mut seed)-0.5)*99.)+b*((rnd(&mut seed)-0.5)*99.);
                            let orig = orig0 + t;
                            let js = 16.;
                            let jt = -1.0;
                            let ja = js * x as f32 * ar + rnd(&mut seed);
                            let jb = js * y as f32 * ar + rnd(&mut seed);
                            let jc = js;
                            let dir = !(t*jt + a*ja + b*jb + c*jc);
                            let s = sampler((*objects).as_slice(),orig,dir,&mut seed);
                            p = s * 3.5 + p ;
                        }
                        (*bytes)[k] = clamp(p.x);
                        k += 1;
                        (*bytes)[k] = clamp(p.y);
                        k += 1;
                        (*bytes)[k] = clamp(p.z);
                        k += 1;
                    }
                }
            }));
        }
        loop{
            let _ = match results.pop()
            {
                Some(t) => t.join(),
                None => break
            };
        }
        let t1 = t0.elapsed();
        res.samples[i_times as usize] = (t1.as_secs()*1000000000 + t1.subsec_nanos() as u64)as f64 / 1000000000.0;
        println!("{}{}{}","Time taken for render ",res.samples[i_times as usize],"s");
    }
    println!("{}{}{}","Average time taken ",res.average(),"s");
    let output_filename = opts.output_filename.clone();
    let output_path:&path::Path = path::Path::new(&output_filename);
    let mut output_writer = File::create(&output_path).unwrap();
    let mut s = String::from_str("P6 ").unwrap();
    s .push_str(&image_size.to_string()[..]);
    s .push_str(" ");
    s .push_str(&image_size.to_string()[..]);
    s .push_str(" 255 ");
    output_writer.write(s.as_bytes());
    unsafe {
    output_writer.write((*bytes.0).as_slice());
    }
    
    let result_filename = opts.result_filename.clone();
    let result_path:&path::Path = path::Path::new(&result_filename);
    let mut writer = &mut File::create(&result_path).unwrap();
    writer.write(format!("{}",json::as_pretty_json(&res.to_json())).as_bytes());
}

fn sampler(objects:&[Vector],o:Vector,d:Vector,seed:&mut usize)->Vector
{
    let tr = tracer(objects,o,d);
    
    if tr.m == Status::KMissUpward {
        let p = 1.0 - d.z;
        return Vector::new_args(1.0,1.0,1.0)*p;
    }
    let on = tr.n;
    let mut h = o+d*tr.t;
    let l = !(Vector::new_args(9.0+rnd(seed),9.0+rnd(seed),16.0)+h*-1.0);
    let mut b = l % tr.n;
    if b < 0.0 {
        b = 0.0;
    } else {
        let tr2 = tracer(objects,h,l);
        if tr2.m != Status::KMissUpward {
            b = 0.0;
        }
    }
    if tr.m == Status::KMissDownward {
        h = h * 0.2;
        b = b * 0.2 + 0.1;
        let chk = (h.x.ceil()+h.y.ceil()) as isize & 1;
        let bc = if 0 != chk {
            Vector::new_args(3.0,1.0,1.0)
        } else {
            Vector::new_args(3.0,3.0,3.0)
        };
        return bc * b;
    }
    let r = d + on * ( on % d * -2.0);
    let p = (l%r * if b > 0.0 {1.0f32} else {0.0f32}).powf(99.0);
    return Vector::new_args(p,p,p)+sampler(objects,h,r,seed)*0.5;
}

fn tracer(objects:&[Vector],o:Vector,d:Vector)->TraceResult
{
    let mut tr = TraceResult{ n:Vector::new_args(0.0,0.0,1.0),m:Status::KMissUpward,t:1e9f32};
    let p = -o.z / d.z;
    if 0.1 < p {
        tr.t = p;
        tr.n = Vector::new_args(0.0,0.0,1.0);
        tr.m = Status::KMissDownward;
    }
    for obj in objects.iter() {
        let p = o + *obj;
        let b = p % d;
        let c = p % p - 1.0;
        let b2 = b * b;
        if b2 > c {
            let q = b2 - c;
            let s = -b - q.sqrt();
            if s < tr.t && s > 0.01 {
                tr.t = s;
                tr.n = !(p+d*tr.t);
                tr.m = Status::KHit;
            }
        }
    }    
    tr
}

fn clamp(v:f32)->u8 {
    if v > 255.0 {
        255
    } else {
        v as u8
    }
}

#[derive(PartialEq)]
enum Status{
    KMissUpward,
    KMissDownward,
    KHit
}

struct TraceResult{
    n: Vector,
    m: Status,
    t: f32
}

fn rnd(seed:&mut usize)->f32 {
    *seed += *seed;
    *seed ^= 1;
    if (*seed as isize) < 0
    {
        *seed ^= 0x88888eef;
    }
    (*seed % 95) as f32 * (1.0 / 95.0) 
}

struct ResultJ{
    samples: Vec<f64>
}

impl ResultJ{
    fn new(times:usize)->ResultJ
    {
        ResultJ {
            samples: from_elem(times,0.0)
        }
    }
    fn average(&self)->f64
    {
        self.samples.iter().fold(0.0, |a,&b| a+b)/self.samples.len() as f64
    }
}

impl json::ToJson for ResultJ{
    fn to_json(&self)->json::Json
    {
        let mut object = BTreeMap::new();
        object.insert(FromStr::from_str("average").unwrap(),self.average().to_json());
        object.insert(FromStr::from_str("samples").unwrap(),self.samples.to_json());
        Json::Object(object)
    }
}

#[derive(Clone)]
struct Vector{
    x:f32,y:f32,z:f32
}

impl Copy for Vector {}

impl Vector{
/*
    fn new()->Vector{
        Vector{x:0.0,y:0.0,z:0.0}
    }*/
    fn new_args(x:f32,y:f32,z:f32)->Vector{
        Vector{x:x,y:y,z:z}
    }
}

impl Add<Vector> for Vector{
    type Output = Vector;
    fn add(self,rhs:Vector)->Vector{
        Vector::new_args(self.x+rhs.x,self.y+rhs.y,self.z+rhs.z)
    }
}

impl Mul<f32> for Vector{
    type Output = Vector;
    fn mul(self,rhs:f32)->Vector{
        Vector::new_args(self.x* rhs,self.y* rhs,self.z* rhs)
    }
}

impl Rem<Vector> for Vector{
    type Output = f32;
    fn rem(self,rhs:Vector)->f32{
        self.x*rhs.x+self.y*rhs.y+self.z*rhs.z
    }
}

impl BitXor<Vector> for Vector{
    type Output = Vector;
    fn bitxor(self,rhs:Vector)->Vector{
        Vector::new_args
            (self.y*rhs.z - self.z*rhs.y,
             self.z*rhs.x - self.x*rhs.z,
             self.x*rhs.y - self.y*rhs.x
            )
    }
}

impl Not for Vector{
    type Output = Vector;
    fn not(self)->Vector{
        self * (1.0/(self % self).sqrt())
    }
}

fn read_art(reader: &mut Read)->Art
{
    let mut art=Vec::new();
    let mut tmp = Vec::new();
    reader.read_to_end(&mut tmp).unwrap();
    let tmp = str::from_utf8(tmp.as_slice());
    for i in tmp.unwrap().lines() {
        art.push(FromStr::from_str(i).unwrap());
    }
    art
}

fn make_objects(art:&Art)->Objects 
{
    let ox = 0.0f32;
    let oy = 6.5f32;
    let oz = -1.0f32;
    
    let mut o = Vec::new();
    let y = oy;
    let mut z = oz - art.len() as f32;
    for line in art.iter() {
        let mut x = ox;
        for c in line.chars() {
            if c != ' ' {
                o.push(Vector::new_args(x,y,z));
            }
            x += 1.0;
        }
        z += 1.0;
    }
    o
}

#[derive(Clone)]
struct Options{
    mega_pixels:f64,
    times: isize,
    procs: isize,
    output_filename:String,
    result_filename:String,
    art_filename:String,
    home:String
}

impl Options{
    fn new()->Options{
        let args: Vec<_> = std::env::args().collect();
        let mut o = Options{
            mega_pixels:1.0,
            times:1,
            procs:32,
            output_filename:FromStr::from_str("render.ppm").unwrap(),
            result_filename:FromStr::from_str("result.json").unwrap(),
            art_filename:FromStr::from_str("ART").unwrap(),
            home: match std::env::var("RAYS_HOME") {
                    Ok(x) => x,
                    Err(_) => FromStr::from_str("").unwrap()
                  }
        };
        let mut options = getopts::Options::new();
            options.optopt("","mp","","");
            options.optopt("t","","","");
            options.optopt("p","","","");
            options.optopt("o","","","");
            options.optopt("r","","","");
            options.optopt("a","","","");
            options.optopt("","home","","");

        let matches = match options.parse(args.as_slice()) {
            Ok(m) => { m }
            Err(f) => { panic!("{}",f.to_string()) }
        };
        match matches.opt_str("mp") {
            Some(x) => { o.mega_pixels = FromStr::from_str(&x[..]).unwrap(); }
            None => ()
        };
        match matches.opt_str("t") {
            Some(x) => { o.times = FromStr::from_str(&x[..]).unwrap(); }
            None => ()
        };
        match matches.opt_str("p") {
            Some(x) => { o.procs = FromStr::from_str(&x[..]).unwrap(); }
            None => ()
        };
        match matches.opt_str("o") {
            Some(x) => { o.output_filename = x; }
            None => ()
        };
        match matches.opt_str("r") {
            Some(x) => { o.result_filename = x; }
            None => ()
        };
        match matches.opt_str("a") {
            Some(x) => { o.art_filename = x; }
            None => ()
        };
        match matches.opt_str("home") {
            Some(x) => { o.home = x; }
            None => ()
        };
        o
    }
}

fn from_elem<T:Clone>(num:usize,e:T)-> Vec<T> {
	let mut v = Vec::new();
	v.resize(num,e);
	return v;
}
