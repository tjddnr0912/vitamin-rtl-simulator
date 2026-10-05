module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'b1100; m = 9;
    case (v) inside
      8'b0000_1?00: m = 1;
      8'h1F: m = 2;
      [8'd5:8'd6]: m = 3;
      8'b1???_??11: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1000; m = 9;
    case (v) inside
      8'b0000_1?00: m = 1;
      8'h1F: m = 2;
      [8'd5:8'd6]: m = 3;
      8'b1???_??11: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1111; m = 9;
    case (v) inside
      8'b0000_1?00: m = 1;
      8'h1F: m = 2;
      [8'd5:8'd6]: m = 3;
      8'b1???_??11: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd5; m = 9;
    case (v) inside
      8'b0000_1?00: m = 1;
      8'h1F: m = 2;
      [8'd5:8'd6]: m = 3;
      8'b1???_??11: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd6; m = 9;
    case (v) inside
      8'b0000_1?00: m = 1;
      8'h1F: m = 2;
      [8'd5:8'd6]: m = 3;
      8'b1???_??11: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0011; m = 9;
    case (v) inside
      8'b0000_1?00: m = 1;
      8'h1F: m = 2;
      [8'd5:8'd6]: m = 3;
      8'b1???_??11: m = 4;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
