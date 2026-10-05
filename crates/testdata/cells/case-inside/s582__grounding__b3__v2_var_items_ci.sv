module top;
  logic [3:0] v; int m; bit [3:0] k2; int ki; logic [3:0] k4;
  initial begin
    k2 = 4'd3; ki = 5; k4 = 4'd7;
    for (int i = 0; i < 9; i++) begin
      v = i[3:0];
      case (v) inside k2: m = 1; ki: m = 2; k4, [k2+4'd5:4'd9]: m = 3; default: m = 0; endcase
      $display("v=%0d m=%0d", v, m);
    end
    #10 $finish;
  end
endmodule
