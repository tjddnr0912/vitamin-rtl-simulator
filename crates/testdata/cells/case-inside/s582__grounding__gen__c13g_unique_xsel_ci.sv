module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'bx001; m = 9;
    unique case (v) inside
      4'd1: m = 1;
      4'b1?00: m = 2;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1x00; m = 9;
    unique case (v) inside
      4'd1: m = 1;
      4'b1?00: m = 2;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
