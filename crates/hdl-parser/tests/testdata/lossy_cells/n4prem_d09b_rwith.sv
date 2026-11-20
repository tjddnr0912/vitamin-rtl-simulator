class C; rand logic [99:0] a; endclass module top; C o; initial begin o = new; o.randomize() with { a <= 'z; }; end endmodule
