module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'd5; m = 9;
    priority case (v) inside
      [4'd0:4'd7]: m = 1;
      4'd5: m = 2;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
