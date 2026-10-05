`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
package p;
`ifdef K3
  parameter logic [3:0] PP = 4'b1x00;
`endif
  parameter logic [3:0] PK = 4'b1100;
endpackage
module sub #(parameter logic [3:0] P = 4'b0000) (input logic [3:0] v, output logic o);
  assign o = `IN(v, P);
endmodule
module t #(parameter logic [3:0] TP = 4'b0000);
  logic [3:0] v1100, v0100; logic [35:0] v36;
  logic o1, o2, o3, o4;
  typedef enum logic [3:0] { EA = 4'b1100, EB = 4'b0011 } en_t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } cs_t;
  localparam cs_t CS = '{a: 4'b1100, b: 4'b0011};
  localparam logic [3:0] CA [0:1] = '{4'b1100, 4'b0011};
  localparam logic signed [3:0] PS = -4;
  function automatic logic [3:0] cf(); return 4'b1100; endfunction
  sub #(.P(4'b1100)) u2(.v(v1100), .o(o2));
`ifdef K1
  sub #(.P(4'b1x00)) u1(.v(v1100), .o(o1));
  sub #(.P(4'b1x00)) u3(.v(v0100), .o(o3));
`endif
`ifdef K2
  sub u4(.v(v1100), .o(o4));
  defparam u4.P = 4'b1x00;
`endif
`ifdef K4
  class C #(parameter logic [3:0] CP = 4'b1x00);
    static function bit f(logic [3:0] v); return `IN(v, CP); endfunction
  endclass
`endif
`ifdef K5
  let LP = 4'b1x00;
  let LU = 'bx1;
`endif
`ifdef K7
  localparam cs_t CX = '{a: 4'b1x00, b: 4'b0000};
`endif
`ifdef K8
  localparam logic [3:0] CAX [0:1] = '{4'b1x00, 4'b0000};
`endif
`ifdef K9
  localparam logic [3:0] QX = 4'd1 / 4'd0;
`endif
  for (genvar i = 0; i < 2; i++) begin : g
    initial #2 $display("Z12_%0d %b", i, `IN(i, 1));
  end
  initial #1000 $finish;
  initial begin
    v1100 = 4'b1100; v0100 = 4'b0100; v36 = 36'hF_0000_0001;
    #1;
    $display("Z01 %b", `IN(v1100, p::PK));
    $display("Z02 %b", `IN(v1100, EA));
    $display("Z03 %b", `IN(v1100, CS.a));
    $display("Z04 %b", `IN(v1100, CA[0]));
    $display("Z05 %b", `IN(v1100, cf()));
    $display("Z06 %b", `IN(4'd4, $bits(v1100)));
    $display("Z07 %b", `IN(4'd4, $clog2(16)));
    $display("Z08 %b", `IN(v1100, (4'b1100)));
    $display("Z09 %b", `IN(v1100, PS));
    $display("Z10 %b", `IN(8'hFC, PS));
    $display("Z11 %b", `IN(-8'sd4, PS));
    $display("Z13 %b", o2);
    $display("Z14 %b TP=%b", `IN(v1100, TP), TP);
`ifdef K1
    $display("K1a %b", o1); $display("K1b %b", o3); $display("K1c %b", `IN(v1100, u1.P));
`endif
`ifdef K2
    $display("K2 %b", o4);
`endif
`ifdef K3
    $display("K3 %b", `IN(v1100, p::PP));
`endif
`ifdef K4
    $display("K4 %b", C#()::f(v1100));
`endif
`ifdef K5
    $display("K5a %b", `IN(v1100, LP)); $display("K5b %b", `IN(v36, LU));
`endif
`ifdef K7
    $display("K7 %b", `IN(v1100, CX.a));
`endif
`ifdef K8
    $display("K8 %b", `IN(v1100, CAX[0]));
`endif
`ifdef K9
    $display("K9 %b", `IN(v1100, QX));
`endif
  end
endmodule
