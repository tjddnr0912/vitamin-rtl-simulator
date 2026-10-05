module top;
  logic [7:0] v8; logic [3:0] a4, b4;
  logic signed [7:0] s8; logic signed [3:0] sa, sb;
  int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("p3 v8=10 in {a4+b4} = %0d", m);
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("p4 v8=10 in {[a4+b4:FF]} = %0d", m);
    case (v8) inside a4 << 1: m = 1; default: m = 0; endcase $display("p4b v8=10 in {a4<<1} = %0d", m);
    v8 = 8'hF7;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("p5 F7 in {~a4} = %0d", m);
    v8 = 8'h07;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("p6 07 in {~a4} = %0d", m);
    v8 = 8'h00;
    case (v8) inside {a4 + b4}: m = 1; default: m = 0; endcase $display("p7 00 in {{a4+b4}} = %0d", m);
    v8 = 8'h10;
    case (v8) inside {a4 + b4}: m = 1; default: m = 0; endcase $display("p8 10 in {{a4+b4}} = %0d", m);
    sa = -4'sd8; sb = -4'sd8; s8 = -8'sd16;
    case (s8) inside sa + sb: m = 1; default: m = 0; endcase $display("s1 -16 in {sa+sb} = %0d", m);
    s8 = 8'sd8;
    case (s8) inside -sa: m = 1; default: m = 0; endcase $display("s2 8 in {-sa} = %0d", m);
    s8 = -8'sd16;
    case (s8) inside [sa + sb : 8'sd0]: m = 1; default: m = 0; endcase $display("s3 -16 in {[sa+sb:0]} = %0d", m);
    s8 = 8'sd0;
    case (s8) inside signed'(sa + sb): m = 1; default: m = 0; endcase $display("s4 0 in {signed'(sa+sb)} = %0d", m);
    s8 = -8'sd16;
    case (s8) inside signed'(sa + sb): m = 1; default: m = 0; endcase $display("s5 -16 in {signed'(sa+sb)} = %0d", m);
    // if-twins
    v8 = 8'h10; $display("t3 if v8=10 inside {a4+b4} = %0d", v8 inside {a4 + b4});
    v8 = 8'h00; $display("t7 if 00 inside {{a4+b4}} = %0d", v8 inside {{a4 + b4}});
    s8 = -8'sd16; $display("t1 if -16 inside {sa+sb} = %0d", s8 inside {sa + sb});
    s8 = -8'sd16; $display("t5 if -16 inside {signed'(sa+sb)} = %0d", s8 inside {signed'(sa + sb)});
    #10 $finish;
  end
endmodule
