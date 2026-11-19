module t; logic [69:0] v; integer r1, r2, r3, r4, r5;
  initial begin
    v = 70'h0;
    case (v) 'x: r1 = 1; default: r1 = 0; endcase
    casez (v) 'z: r2 = 1; default: r2 = 0; endcase
    casex (v) 'x: r3 = 1; default: r3 = 0; endcase
    v = {70{1'bx}};
    case (v) 'x: r4 = 1; default: r4 = 0; endcase
    v = {6'b0, {64{1'bx}}};
    case (v) 'x: r5 = 1; default: r5 = 0; endcase
    #1 $display("A r1=%0d r2=%0d r3=%0d r4=%0d r5=%0d", r1, r2, r3, r4, r5);
  end
  initial #100 $finish;
endmodule
