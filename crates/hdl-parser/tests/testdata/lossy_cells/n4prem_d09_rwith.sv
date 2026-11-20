class C; rand logic [99:0] a; endclass module top; C o; initial begin o = new; void'(o.randomize() with { a <= 'z; }); end endmodule
