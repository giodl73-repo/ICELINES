<!-- proof:compiled from="proof:slides" count=11 -->
```slides
SLIDE 1 ─────────────────────────────────────────────────────────────────────── 1/11
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                    IceLines                                    
                       NHL analytics + fantasy — Rust CLI                       
                                                                                
                                    icelines                                    
                                      2026                                      
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 2 ─────────────────────────────────────────────────────────────────────── 2/11
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                            ── What IceLines Does ──                            
                                                                                
                   Five seasons of data, zero fetch required                    
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 3 ─────────────────────────────────────────────────────────────────────── 3/11
Commands                                                                        
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 4 ─────────────────────────────────────────────────────────────────────── 4/11
                                                                                
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
● icelines query leaders — pace-adjusted leaderboard                            
● icelines team depth TB — full line/pair/pair chart                            
● icelines fantasy gaps/simulate — roster gaps and add/drop projections         
● icelines export md — markdown tables for proof/mdpath                         
● icelines comps HEDMAN — cross-team comparisons                                
● ```                                                                           
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 5 ─────────────────────────────────────────────────────────────────────── 5/11
Data model                                                                      
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 6 ─────────────────────────────────────────────────────────────────────── 6/11
                                                                                
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
● Five seasons bundled in binary (no fetch required)                            
● Real-time: optional NHL API fetch adds current season                         
● MoneyPuck: optional advanced stats integration                                
● Snapshot store: daily snapshots, diff queries                                 
                                                                                
────────────────────────────────────────────────────────────────────────────────
                                                                                
All data sources go through icelines-fetch.                                     
Business logic stays in icelines-core.                                          
```                                                                             
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 7 ─────────────────────────────────────────────────────────────────────── 7/11
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 8 ─────────────────────────────────────────────────────────────────────── 8/11
                                                                                
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
proof:stat label="Seasons" value="5" delta=""                                   
proof:stat label="Players" value="~900" delta=""                                
proof:stat label="Stats" value="30+" delta=""                                   
proof:stat label="Commands" value="12" delta=""                                 
```                                                                             
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 9 ─────────────────────────────────────────────────────────────────────── 9/11
proof integration                                                               
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 10 ────────────────────────────────────────────────────────────────────── 10/11
                                                                                
                                                                                
                                                                                
────────────────────────────────────────────────────────────────────────────────
ℹ IceLines exports markdown tables that proof:tree and proof:element            
  can consume via md:// URIs — connecting analytics data to                     
  documentation compilation.                                                    
                                                                                
● icelines export md → src/data/players.md                                      
● proof:tree source=md://src/data/players.md — taxonomy of roster               
● proof:element source=md://src/data/players.md field=points — scoreboard       
● ```                                                                           
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
SLIDE 11 ────────────────────────────────────────────────────────────────────── 11/11
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                    IceLines                                    
                        github.com/giodl73-repo/ICELINES                        
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
```
<!-- /proof:compiled -->
