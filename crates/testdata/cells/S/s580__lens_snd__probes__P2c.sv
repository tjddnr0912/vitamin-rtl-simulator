// P2c: more const consumers of F1 + S2 x/z values reaching an element through a parameter
module sub #(parameter P = 0) ();
  initial #1 $display("OVR P=%0d", P);
endmodule
module sub2 #(parameter logic [3:0] P = 4'b0000) (input logic [3:0] v);
  initial #1 $display("OVX v inside {P} = %b", v inside {P});
endmodule
module t;
  localparam bit LB = (4'd0 - 4'd1) inside {5'b1111?};  // 1
  sub #(.P((4'd15 + 4'd1) inside {5'b1?000})) u_ov();    // P=1
  sub2 #(.P(4'b1x00)) u_x(.v(4'b1100));                  // 1
  initial begin
    $display("LB=%b", LB);
    #2 $finish;
  end
endmodule
