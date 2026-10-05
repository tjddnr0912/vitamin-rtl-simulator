module top;
  logic [7:0] a, b, v8; logic [3:0] a4, b4; int m;
  initial begin
    a = 8'h80; b = 8'h80; a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (a + b) inside 9'h100: m = 1; default: m = 0; endcase $display("p1 (a+b) in {9'h100} = %0d", m);
    case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("p3 v8 in {a4+b4} = %0d", m);
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("p4 v8 in {[a4+b4:FF]} = %0d", m);
    v8 = 8'hF7; a4 = 4'h8;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("p5 F7 in {~a4} = %0d", m);
    v8 = 8'h07;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("p6 07 in {~a4} = %0d", m);
    #10 $finish;
  end
endmodule
