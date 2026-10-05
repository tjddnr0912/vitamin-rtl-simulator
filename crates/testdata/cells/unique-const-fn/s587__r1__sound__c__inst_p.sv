module child (input logic [f(2):0] p);
  function automatic int f(input int a); return 3; endfunction
  logic [f(2):0] q;
  assign q = p;
  initial #1 $display("%m bp=%0d p=%0d bq=%0d", $bits(p), p, $bits(q));
endmodule
module top;
  function automatic int f(input int a);
    if (a == 1) return 1;
    return 7;
  endfunction
  logic [15:0] bus = 16'd1000;
  child u (.p(bus));
  initial #2 begin $display("hb=%0d hq=%0d hp=%0d", $bits(u.q), u.q, u.p); $finish; end
endmodule
