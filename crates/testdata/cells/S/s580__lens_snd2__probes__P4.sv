`timescale 1ns/1ns
package pk;
  function automatic bit pf(input logic [35:0] a); return a inside {'bx1}; endfunction
  function automatic bit pg(input logic [35:0] a); return a inside {32'bx1}; endfunction
endpackage
`ifndef NOCLASS
class C;
  function bit m(input logic [35:0] a); return a inside {'bx1}; endfunction
  function bit n(input logic [35:0] a); return a inside {32'bx1}; endfunction
endclass
`endif
module t;
  logic [35:0] v = 36'hF_0000_0001;
  wire c1 = v inside {'bx1};
  wire c2 = v inside {32'bx1};
  wire [7:0] c3 = 8'(v inside {'bx1});
  wire [7:0] c4 = 8'(v inside {32'bx1});
  wire c5 = (v ==? 'bx1);
  wire c6 = (v ==? 32'bx1);
  function automatic bit f(input logic [35:0] a); return a inside {'bx1}; endfunction
  function automatic bit g(input logic [35:0] a); return (a inside {('bx1)}) && (a ==? ('bx1)); endfunction
  task automatic tk(input logic [35:0] a); $display("T %b %b", a inside {'bx1}, a inside {32'bx1}); endtask
  for (genvar i = 0; i < 1; i++) begin : gb
    wire gc1 = v inside {'bx1};
    wire gc2 = v inside {32'bx1};
  end
`ifndef NOCLASS
  C o;
`endif
  bit r1, r2;
  initial begin
`ifndef NOCLASS
    o = new;
`endif
    #1;
    r1 = v inside {'bx1}; r2 = v inside {32'bx1};
    $display("R %b %b %b %b", r1, r2, (v inside {'bx1}) ? 1'b1 : 1'b0, (v inside {32'bx1}) ? 1'b1 : 1'b0);
    $display("C %b %b %b %b %b %b", c1, c2, c3, c4, c5, c6);
    $display("F %b %b %b %b", f(v), g(v), pk::pf(v), pk::pg(v));
`ifndef NOCLASS
    $display("M %b %b", o.m(v), o.n(v));
`endif
    tk(v);
    $display("G %b %b", gb[0].gc1, gb[0].gc2);
    $display("N %b %b", v !=? 'bx1, v !=? 32'bx1);
    #1 $finish;
  end
endmodule
