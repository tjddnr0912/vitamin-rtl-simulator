`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module sub #(parameter logic [3:0] P = 4'b0000, parameter int ID = 0) (input logic [3:0] v, output logic o);
  assign o = `IN(v, P);
  initial #2 $display("PV%0d %b", ID, P);
endmodule
module t;
  logic [3:0] v1100; logic [35:0] v36;
  logic o1;
  sub #(.P(4'b1x00), .ID(1)) u1(.v(v1100), .o(o1));
  localparam L20 = `IN(4'd15 + 4'd1, 8'b0000_?000);
  localparam L21 = `IN(4'hF << 1, 8'b0000_111?);
  localparam L22 = `IN(~4'b0011, 8'b0000_11?0);
  if (`IN(4'd15 + 4'd1, 8'b0000_?000)) begin : g20 initial #2 $display("G20 then"); end
  else begin : g20e initial #2 $display("G20 else"); end
  logic [`IN(4'd15 + 4'd1, 8'b0000_?000) ? 7 : 3 : 0] ab;
`ifdef LETT
  let LU = 'bx1;
  let LP = 4'b1x00;
`endif
`ifdef ARR
  localparam logic [3:0] CAX [0:1] = '{4'b1x00, 4'b0000};
`endif
  initial #1000 $finish;
  initial begin
    v1100 = 4'b1100; v36 = 36'hF_0000_0001;
    #1;
    $display("L20 %b", L20); $display("L21 %b", L21); $display("L22 %b", L22);
    $display("AB %0d", $bits(ab));
    $display("R20 %b", `IN(4'd15 + 4'd1, 8'b0000_?000));
    $display("K1a %b", o1);
    $display("K1c %b", `IN(v1100, u1.P));
`ifdef LETT
    $display("K5a %b", `IN(v1100, LP));
    $display("K5b %b", `IN(v36, LU));
    $display("K5c %b", `IN(v36, 'bx1));
`endif
`ifdef ARR
    $display("K8 %b", `IN(v1100, CAX[0]));
    $display("K8v %b", CAX[0]);
`endif
  end
endmodule
