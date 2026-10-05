`timescale 1ns/1ns
`ifdef IFC
interface ifc;
  function automatic bit f(logic [3:0] v); return v inside {4'b1?00}; endfunction
  task automatic tk(input logic [3:0] v, output bit r); r = v inside {4'b1?00}; endtask
endinterface
`endif
module sub(input logic i, output logic o); assign o = i; endmodule
module t;
  logic [3:0] v, n8, n9, n10, n26, n33, n34; logic r20, r24, r25, o16, w15, l29, en;
  logic [3:0] a9 [0:3]; logic [3:0] arr22 [0:1];
  typedef struct packed { logic a; logic b; } s2_t; s2_t st23;
  string s13;
  localparam LB = (4'b1100 inside {4'b1?00}) ? 7 : 3;
  logic [LB:0] a19;
  assign #1 w15 = v inside {4'b1?00};
  sub u16(.i(v inside {4'b1?00}), .o(o16));
  always_latch if (en) l29 = v inside {4'b1?00};
  case (4'b1100 inside {4'b1?00})
    1'b1: begin : gc1 initial #3 $display("C18 one"); end
    default: begin : gc0 initial #3 $display("C18 dflt"); end
  endcase
`ifdef IFC
  ifc i1();
  bit r1, r2;
`endif
`ifdef FRC
  logic w21;
`endif
`ifdef FJ
  logic r6;
`endif
  initial #1000 $finish;
`ifdef FIN
  final $display("C05 %b", v inside {4'b1?00});
`endif
  initial begin
    v = 4'b1100; en = 1; n8 = 0; n9 = 0; n10 = 0; n26 = 0; n33 = 0; n34 = 0;
    a9[0] = 12; a9[1] = 8; a9[2] = 4; a9[3] = 0; arr22[0] = 5; arr22[1] = 9;
    #2;
    do n8++; while ((n8 + 4'd0) inside {4'b000?});
    foreach (a9[i]) if (a9[i] inside {4'b1?00}) n9++;
    repeat ((v inside {4'b1?00}) ? 3 : 1) n10++;
    case (v inside {4'b1?00}) 1'b1: r20 = 1; default: r20 = 0; endcase
    st23 = '{a: v inside {4'b1?00}, b: 1'b0};
    unique if (v inside {4'b1?00}) r24 = 1; else r24 = 0;
    begin : blk25 r25 = 1; if (v inside {4'b1?00}) disable blk25; r25 = 0; end
    while (1) begin n26++; if ((n26 + 4'd0) inside {4'b01??}) break; end
    for (int i = (v inside {4'b1?00}) ? 2 : 0; i < 3; i++) n33++;
    n34 += v inside {4'b1?00};
    s13 = $sformatf("%b", v inside {4'b1?00});
    $display("C08 %0d", n8); $display("C09 %0d", n9); $display("C10 %0d", n10);
    $display("C11 %b", ((v inside {4'b1?00}) inside {1'b1}));
    $display("C11b %b", ((v inside {4'b1?00}) inside {1'b?}));
    $display("C11c %b", ((v inside {4'b0?00}) inside {1'b0}));
    $display("C12 %0d", $countones({v inside {4'b1?00}, 3'b101}));
    $display("C13 %s", s13);
    $display("C15 %b", w15); $display("C16 %b", o16);
    $display("C19 %0d", $bits(a19)); $display("C20 %b", r20);
    $display("C22 %0d", arr22[v inside {4'b1?00}]);
    $display("C23 %b", st23.a); $display("C24 %b", r24); $display("C25 %b", r25);
    $display("C26 %0d", n26); $display("C29 %b", l29); $display("C33 %0d", n33); $display("C34 %0d", n34);
`ifdef IFC
    r1 = i1.f(4'b1000); i1.tk(4'b1100, r2);
    $display("C01 %b", r1); $display("C02 %b", r2);
`endif
`ifdef FRC
    force w21 = v inside {4'b1?00}; #1 $display("C21 %b", w21);
`endif
`ifdef FJ
    fork begin #1 r6 = v inside {4'b1?00}; end begin #5; end join_any
    $display("C06 %b", r6);
`endif
  end
endmodule
