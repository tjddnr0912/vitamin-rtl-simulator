module top;
  logic [3:0] v; logic m;
  task automatic t(input logic [3:0] x);
    v = x;
    case (v) inside
      4'b1?00: m = 1;
      [4'd1:4'd3]: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
  endtask
  initial begin
    #1 t(4'b1000); t(4'b0010); t(4'b0110); t(4'b1100); t(4'b0000); t(4'b0011); t(4'b0100);
    #100 $finish;
  end
endmodule
