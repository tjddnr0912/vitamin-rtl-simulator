module top;
  function automatic logic [3:0] hm(input int x);
    logic [3:0] t;
    hm = t;
  endfunction
  logic [31:0] v = 32'h89abcdef;
  logic [31:0] o;
  initial begin o = v[0 +: hm(2)]; $display("o=%h", o); #1 $finish; end
  initial #50 $finish;
endmodule
