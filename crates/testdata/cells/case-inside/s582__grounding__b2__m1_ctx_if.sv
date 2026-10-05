module top;
  logic [7:0] a, b, v8; logic [3:0] a4, b4; int m;
  initial begin
    a = 8'h80; b = 8'h80; a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    m = ((a + b) inside {9'h100}); $display("p1 (a+b) in {9'h100} = %0d", m);
    m = ((a + b) == 9'h100); $display("p2 (a+b) == 9'h100 = %0d", m);
    m = (v8 inside {a4 + b4}); $display("p3 v8 in {a4+b4} = %0d", m);
    m = (v8 inside {[a4 + b4 : 8'hFF]}); $display("p4 v8 in {[a4+b4:FF]} = %0d", m);
    v8 = 8'hF7; a4 = 4'h8;
    m = (v8 inside {~a4}); $display("p5 F7 in {~a4} = %0d", m);
    v8 = 8'h07;
    m = (v8 inside {~a4}); $display("p6 07 in {~a4} = %0d", m);
    #10 $finish;
  end
endmodule
